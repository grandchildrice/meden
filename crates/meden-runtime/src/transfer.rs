//! Immutable HTTP export and one-use reservation before actual delivery.
//! The host owns approval. This component is not yet connected to TUI tools.

use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use meden_core::transfer::{self, Permit, Request};

/// One prepared POST. Target and payload cannot change after preparation.
/// No constructor, inspection or approval operation performs network I/O.
pub struct Export {
    target: String,
    payload: Vec<u8>,
    request: Request,
    created: Instant,
    permit: Mutex<Option<Permit>>,
}

impl Export {
    /// Freeze a complete request. HTTPS is required except for literal local
    /// loopback receivers; an HTTP redirect is never followed automatically.
    pub fn prepare(session: &[u8], target: &str, payload: &[u8]) -> io::Result<Self> {
        let uri: ureq::http::Uri = target
            .parse()
            .map_err(|_| io::Error::other("invalid transfer URI"))?;
        let local = matches!(uri.host(), Some("127.0.0.1" | "[::1]"));
        if uri
            .authority()
            .is_none_or(|authority| authority.as_str().contains('@'))
            || !(uri.scheme_str() == Some("https") || local && uri.scheme_str() == Some("http"))
        {
            return Err(io::Error::other(
                "transfer requires HTTPS or literal loopback HTTP, without userinfo",
            ));
        }
        let target = uri.to_string();
        let description = serde_json::to_vec(&serde_json::json!({
            "method": "POST", "target": target,
            "headers": {"content-type":"application/octet-stream"}, "body": payload,
        }))?;
        Ok(Self {
            request: Request {
                session: session.to_vec(),
                recipient: target.as_bytes().to_vec(),
                request: description,
            },
            target,
            payload: payload.to_vec(),
            created: Instant::now(),
            permit: Mutex::new(None),
        })
    }

    /// Actual destination for trusted review, derived from the frozen URI.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Actual bytes for trusted review, independent of later file changes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Issue one approval through the trusted host UI/controller only.
    /// Never register this method as a model tool. An existing approval,
    /// including a spent/expired one, is never reset by this operation.
    pub fn approve_from_host(&self, valid_for: Duration) -> io::Result<()> {
        let duration = u64::try_from(valid_for.as_millis()).map_err(io::Error::other)?;
        let expires = self
            .now()?
            .checked_add(duration)
            .filter(|_| duration > 0)
            .ok_or_else(|| io::Error::other("invalid approval lifetime"))?;
        let mut slot = self
            .permit
            .lock()
            .map_err(|_| io::Error::other("approval lock unavailable"))?;
        if slot.is_some() {
            return Err(io::Error::other(
                "approval already issued; prepare a new export",
            ));
        }
        *slot = Some(Permit {
            request: self.request.clone(),
            expires,
            spent: false,
        });
        Ok(())
    }

    /// Reserve under the mutex, then deliver the same immutable request.
    /// The current session comes from the host, not model arguments. The
    /// returned status is a delivery result, not permission to follow a URL.
    /// Any transport failure leaves approval spent, even on uncertain delivery.
    pub fn send(&self, current_session: &[u8]) -> io::Result<u16> {
        {
            let mut slot = self
                .permit
                .lock()
                .map_err(|_| io::Error::other("approval lock unavailable"))?;
            let approved = slot.as_mut().is_some_and(|permit| {
                transfer::reserve(
                    self.now().unwrap_or(u64::MAX),
                    current_session,
                    permit,
                    &self.request,
                )
            });
            if !approved {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "no matching unspent transfer approval",
                ));
            }
        }
        // A fresh client cannot replay a request from a pooled connection.
        // Disable environment proxies and all automatic redirect hops.
        let config = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(5)))
            .build();
        let agent: ureq::Agent = config.into();
        agent
            .post(&self.target)
            .header("content-type", "application/octet-stream")
            .send(self.payload.as_slice())
            .map(|response| response.status().as_u16())
            .map_err(|_| io::Error::other("delivery failed or uncertain; approval remains spent"))
    }

    fn now(&self) -> io::Result<u64> {
        u64::try_from(self.created.elapsed().as_millis()).map_err(io::Error::other)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Barrier};
    use std::thread::{self, JoinHandle};

    fn listener() -> TcpListener {
        let listener = TcpListener::bind("127.0.0.1:0").expect("receiver");
        listener.set_nonblocking(true).expect("nonblocking");
        listener
    }

    fn target(listener: &TcpListener) -> String {
        format!(
            "http://{}/approved",
            listener.local_addr().expect("address")
        )
    }

    fn receive(listener: &TcpListener, reply: String) -> JoinHandle<Vec<u8>> {
        let listener = listener.try_clone().expect("receiver clone");
        thread::spawn(move || {
            let started = Instant::now();
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(
                            started.elapsed() < Duration::from_secs(5),
                            "no approved delivery"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => return format!("receiver failed: {error}").into_bytes(),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("timeout");
            let mut reader = BufReader::new(stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).expect("header") > 0);
                if line == "\r\n" {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    if key.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().expect("body length");
                    }
                }
            }
            let mut payload = vec![0; length];
            reader.read_exact(&mut payload).expect("body");
            if !reply.is_empty() {
                reader
                    .get_mut()
                    .write_all(reply.as_bytes())
                    .expect("response");
            }
            payload
        })
    }

    fn ok() -> String {
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
    }

    #[test]
    fn no_send_before_approval_and_only_frozen_bytes_reach_the_approved_receiver() {
        let receiver = listener();
        let mut source = b"CONFIDENTIAL-CANARY\0\xff".to_vec();
        let export = Export::prepare(b"session", &target(&receiver), &source).expect("prepare");
        assert_eq!(export.target(), target(&receiver));
        assert_eq!(export.payload(), source);
        assert_eq!(
            export.send(b"session").expect_err("unapproved").kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(receiver.accept().is_err());
        export
            .approve_from_host(Duration::from_secs(60))
            .expect("host approval");
        assert!(export.send(b"other-session").is_err());
        assert!(receiver.accept().is_err());
        source.fill(b'x');
        let observed = receive(&receiver, ok());
        assert_eq!(export.send(b"session").expect("delivery"), 200);
        assert_eq!(
            observed.join().expect("receiver"),
            b"CONFIDENTIAL-CANARY\0\xff"
        );
        assert!(export.send(b"session").is_err());
        assert!(export.approve_from_host(Duration::from_secs(60)).is_err());
        assert!(receiver.accept().is_err());
    }

    #[test]
    fn concurrent_sends_consume_one_approval_once() {
        let receiver = listener();
        let export =
            Arc::new(Export::prepare(b"s", &target(&receiver), b"one copy").expect("prepare"));
        export
            .approve_from_host(Duration::from_secs(60))
            .expect("approval");
        let observed = receive(&receiver, ok());
        let barrier = Arc::new(Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let export = Arc::clone(&export);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    export.send(b"s").is_ok()
                })
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| thread.join().expect("sender"))
                .filter(|sent| *sent)
                .count(),
            1
        );
        assert_eq!(observed.join().expect("receiver"), b"one copy");
        assert!(receiver.accept().is_err());
    }

    #[test]
    fn redirects_do_not_forward_the_payload_to_another_receiver() {
        let approved = listener();
        let other = listener();
        let observed = receive(&approved, format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", target(&other)));
        let export = Export::prepare(b"s", &target(&approved), b"secret").expect("prepare");
        export
            .approve_from_host(Duration::from_secs(60))
            .expect("approval");
        assert_eq!(export.send(b"s").expect("redirect status"), 307);
        assert_eq!(observed.join().expect("receiver"), b"secret");
        assert!(other.accept().is_err());
        assert!(export.send(b"s").is_err());
    }

    #[test]
    fn uncertain_delivery_does_not_restore_approval() {
        let receiver = listener();
        let observed = receive(&receiver, String::new());
        let export = Export::prepare(b"s", &target(&receiver), b"secret").expect("prepare");
        export
            .approve_from_host(Duration::from_secs(60))
            .expect("approval");
        assert!(export.send(b"s").is_err());
        assert_eq!(observed.join().expect("receiver"), b"secret");
        assert!(export.send(b"s").is_err());
        assert!(receiver.accept().is_err());
    }
}
