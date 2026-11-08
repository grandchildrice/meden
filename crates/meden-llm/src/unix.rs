//! HTTP over one pinned, host-prepared Unix socket. No TCP/DNS fallback.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, UnixAddr};
use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport,
};

#[derive(Debug)]
struct UnixConnector(PathBuf);

impl Connector for UnixConnector {
    type Out = UnixTransport;
    fn connect(
        &self,
        details: &ConnectionDetails,
        _: Option<()>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        if details.uri.host() != Some("meden.local")
            || details.needs_tls()
            || details.config.proxy().is_some()
        {
            return Err(ureq::Error::ConnectionFailed);
        }
        // An untrusted engine can replace a pathname in its scratch directory.
        // Pin the socket inode without following symlinks, then connect via the
        // held O_PATH fd. A check-then-connect on the pathname is insufficient.
        let file = pin_socket(&self.0)?;
        let stream = connect_pinned(&file)?;
        Ok(Some(UnixTransport {
            stream,
            buffers: LazyBuffers::new(16_384, 16_384),
        }))
    }
}

fn pin_socket(path: &Path) -> std::io::Result<File> {
    let file = File::options()
        .read(true)
        .custom_flags(nix::libc::O_PATH | nix::libc::O_NOFOLLOW)
        .open(path)?;
    if !file.metadata()?.file_type().is_socket() {
        return Err(std::io::Error::other(
            "local endpoint must be a socket, without symlinks",
        ));
    }
    Ok(file)
}

fn connect_pinned(file: &File) -> Result<UnixStream, ureq::Error> {
    let fd: OwnedFd = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
        None,
    )
    .map_err(std::io::Error::from)?;
    let pinned = UnixAddr::new(format!("/proc/self/fd/{}", file.as_raw_fd()).as_str())
        .map_err(std::io::Error::from)?;
    connect(fd.as_raw_fd(), &pinned).map_err(std::io::Error::from)?;
    let stream = UnixStream::from(fd);
    stream.set_nonblocking(false)?;
    Ok(stream)
}

#[derive(Debug)]
struct NoDns;
impl Resolver for NoDns {
    fn resolve(
        &self,
        _: &ureq::http::Uri,
        _: &ureq::config::Config,
        _: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let mut addresses = self.empty();
        addresses.push(([127, 0, 0, 1], 1).into());
        Ok(addresses)
    }
}

#[derive(Debug)]
struct UnixTransport {
    stream: UnixStream,
    buffers: LazyBuffers,
}
impl Transport for UnixTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }
    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        let timeout = timeout.for_write();
        self.stream.set_write_timeout(duration(timeout)?)?;
        let bytes = self
            .buffers
            .output()
            .get(..amount)
            .ok_or(ureq::Error::ConnectionFailed)?;
        self.stream
            .write_all(bytes)
            .map_err(|error| io_error(error, timeout))
    }
    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        let timeout = timeout.for_read();
        self.stream.set_read_timeout(duration(timeout)?)?;
        let read = self
            .stream
            .read(self.buffers.input_append_buf())
            .map_err(|error| io_error(error, timeout))?;
        self.buffers.input_appended(read);
        Ok(read > 0)
    }
    fn is_open(&mut self) -> bool {
        false
    } // fresh connection for each exchange
}

fn duration(timeout: NextTimeout) -> Result<Option<Duration>, ureq::Error> {
    if timeout.after.is_not_happening() {
        Ok(None)
    } else if timeout.after.is_zero() {
        Err(ureq::Error::Timeout(timeout.reason))
    } else {
        Ok(Some(*timeout.after))
    }
}

fn io_error(error: std::io::Error, timeout: NextTimeout) -> ureq::Error {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        ureq::Error::Timeout(timeout.reason)
    } else {
        error.into()
    }
}

pub(crate) fn agent(socket: &Path, timeout: Duration) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .max_idle_connections(0)
        .timeout_global(Some(timeout))
        .build();
    ureq::Agent::with_parts(config, UnixConnector(socket.to_owned()), NoDns)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_socket_path_cannot_redirect_the_host_after_pinning() {
        let root = tempfile::tempdir().expect("directory");
        let path = root.path().join("model.sock");
        let other = root.path().join("other.sock");
        let original = UnixListener::bind(&path).expect("original");
        let unapproved = UnixListener::bind(&other).expect("other");
        original.set_nonblocking(true).expect("nonblocking");
        unapproved.set_nonblocking(true).expect("nonblocking");
        let pinned = pin_socket(&path).expect("pin socket");
        std::fs::rename(&path, root.path().join("moved.sock")).expect("move");
        std::os::unix::fs::symlink(&other, &path).expect("replacement");
        let _stream = connect_pinned(&pinned).expect("original inode");
        assert!(original.accept().is_ok());
        assert!(unapproved.accept().is_err());
        assert!(
            pin_socket(&path).is_err(),
            "a new symlink is never admitted"
        );
    }

    #[test]
    fn an_unresponsive_local_peer_obeys_the_request_timeout() {
        let root = tempfile::tempdir().expect("directory");
        let path = root.path().join("model.sock");
        let _receiver = UnixListener::bind(&path).expect("receiver");
        let started = std::time::Instant::now();
        let result = agent(&path, Duration::from_millis(30))
            .get("http://meden.local/health")
            .call();
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
