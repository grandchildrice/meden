//! Immutable HTTP export and one-use reservation before actual delivery.
//! The host owns approval; model tools can only propose immutable snapshots.

use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use meden_core::evidence::{ContractView, Contracts, Delivery};
use meden_core::export_control::{Dispatch, ExportEvent, Origin, Phase};

/// One prepared POST. Target and payload cannot change after preparation.
/// No constructor, inspection or approval operation performs network I/O.
pub struct Export {
    state: Mutex<Delivery>,
    created: Instant,
}

/// Host review derived from the same immutable request used by the kernel.
pub struct Snapshot {
    /// Frozen accepted meaning and evidence subject, or an ordinary file.
    pub contract: Option<ContractView>,
    /// Current review/reservation lifecycle.
    pub phase: Phase,
    /// Exact normalized destination.
    pub target: String,
    /// Exact body bytes, independent of later workspace changes.
    pub payload: Vec<u8>,
}

impl Export {
    /// Freeze a complete request. HTTPS is required except for literal local
    /// loopback receivers; an HTTP redirect is never followed automatically.
    pub fn prepare(current: &Contracts, target: &str, payload: &[u8]) -> io::Result<Self> {
        if target.contains('#') || target.len() > 8192 {
            return Err(io::Error::other(
                "transfer URI must be at most 8192 bytes without a fragment",
            ));
        }
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
        Ok(Self {
            state: Mutex::new(
                current
                    .prepare_delivery(target, payload.to_vec())
                    .ok_or_else(|| {
                        io::Error::other("current project evidence is required for export")
                    })?,
            ),
            created: Instant::now(),
        })
    }

    /// Read the kernel's actual frozen request for trusted host presentation.
    pub fn snapshot(&self) -> io::Result<Snapshot> {
        let state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("export state unavailable"))?;
        let post = state.request();
        Ok(Snapshot {
            contract: state.contract().cloned(),
            phase: state.phase(),
            target: post.target().clone(),
            payload: post.body().to_vec(),
        })
    }

    /// Mark inspection through the host channel, never a model tool.
    pub fn review_from_host(&self, current: &Contracts) -> io::Result<()> {
        self.host_transition(current, ExportEvent::Review)
    }

    /// Decline through the host channel, without producing an effect.
    pub fn reject_from_host(&self, current: &Contracts) -> io::Result<()> {
        self.host_transition(current, ExportEvent::Reject)
    }

    fn host_transition(&self, current: &Contracts, event: ExportEvent) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("export state unavailable"))?;
        if !state
            .advance(current, Origin::Host, event, 0, 0, &[])
            .accepted
        {
            return Err(io::Error::other("export transition refused"));
        }
        Ok(())
    }

    /// Host approval and reservation are one kernel transition under one lock.
    /// The executor consumes a dispatch borrowing that same request. None means
    /// delivery failed or is uncertain, with the attempt still consumed. Never expose
    /// this method as a model tool.
    pub fn send(
        &self,
        current: &Contracts,
        current_session: &[u8],
        valid_for: Duration,
    ) -> io::Result<Option<u16>> {
        let duration = u64::try_from(valid_for.as_millis()).map_err(io::Error::other)?;
        let expires = self
            .now()?
            .checked_add(duration)
            .filter(|_| duration > 0)
            .ok_or_else(|| io::Error::other("invalid approval lifetime"))?;
        self.send_before(current, current_session, expires)
    }

    fn send_before(
        &self,
        current: &Contracts,
        current_session: &[u8],
        expires: u64,
    ) -> io::Result<Option<u16>> {
        // Keep the request borrowed and immutable for the bounded HTTP call.
        // Each export has its own lock; the token cannot outlive its owner.
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("export state unavailable"))?;
        let effect = state
            .advance(
                current,
                Origin::Host,
                ExportEvent::Approve,
                self.now()?,
                expires,
                current_session,
            )
            .effect
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "export needs current matching evidence, prior review and valid session approval; no replay",
                )
            })?;
        let sent = Self::dispatch(effect);
        let event = if sent.is_ok() {
            ExportEvent::ResponseReceived
        } else {
            ExportEvent::DeliveryUncertain
        };
        if !state
            .advance(current, Origin::Transport, event, 0, 0, &[])
            .accepted
        {
            return Err(io::Error::other("export result refused"));
        }
        Ok(sent.ok())
    }

    fn dispatch(effect: Dispatch<'_>) -> io::Result<u16> {
        let post = effect.into_request();
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
            .post(post.target())
            .header("content-type", "application/octet-stream")
            .send(post.body())
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
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("request line");
            assert!(request_line.starts_with("POST /approved HTTP/1."));
            let mut length = 0;
            let mut content_type = false;
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
                    if key.eq_ignore_ascii_case("content-type") {
                        assert_eq!(value.trim(), "application/octet-stream");
                        content_type = true;
                    }
                }
            }
            assert!(content_type, "missing frozen content type");
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
        let export = Export::prepare(
            &Contracts::new(b"session".to_vec()),
            &target(&receiver),
            &source,
        )
        .expect("prepare");
        assert_eq!(
            export.snapshot().expect("snapshot").target,
            target(&receiver)
        );
        assert_eq!(export.snapshot().expect("snapshot").payload, source);
        assert_eq!(
            export
                .send(
                    &Contracts::new(b"session".to_vec()),
                    b"session",
                    Duration::from_secs(60)
                )
                .expect_err("unreviewed")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(receiver.accept().is_err());
        let foreign = Export::prepare(
            &Contracts::new(b"session".to_vec()),
            &target(&receiver),
            &source,
        )
        .expect("prepare");
        foreign
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("host review");
        assert!(foreign
            .send(
                &Contracts::new(b"other-session".to_vec()),
                b"other-session",
                Duration::from_secs(60)
            )
            .is_err());
        assert!(foreign
            .send(
                &Contracts::new(b"session".to_vec()),
                b"session",
                Duration::from_secs(60)
            )
            .is_err());
        assert!(receiver.accept().is_err());
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("host review");
        source.fill(b'x');
        let observed = receive(&receiver, ok());
        assert_eq!(
            export
                .send(
                    &Contracts::new(b"session".to_vec()),
                    b"session",
                    Duration::from_secs(60)
                )
                .expect("delivery"),
            Some(200)
        );
        assert_eq!(
            observed.join().expect("receiver"),
            b"CONFIDENTIAL-CANARY\0\xff"
        );
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("inspection cannot reopen");
        assert!(export
            .send(
                &Contracts::new(b"session".to_vec()),
                b"session",
                Duration::from_secs(60)
            )
            .is_err());
        assert!(receiver.accept().is_err());
    }

    #[test]
    fn invalid_targets_and_expired_approvals_never_dispatch() {
        let receiver = listener();
        for invalid in [
            "https://example.invalid/path#hidden",
            "http://remote.example/path",
            "https://user@remote.example/path",
        ] {
            assert!(Export::prepare(&Contracts::new(b"s".to_vec()), invalid, b"secret").is_err());
        }
        let export = Export::prepare(
            &Contracts::new(b"s".to_vec()),
            &target(&receiver),
            b"secret",
        )
        .expect("prepare");
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("review");
        assert!(export
            .send(&Contracts::new(b"s".to_vec()), b"s", Duration::ZERO)
            .is_err());
        assert_eq!(
            export
                .send_before(&Contracts::new(b"s".to_vec()), b"s", 0)
                .expect_err("expired")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(receiver.accept().is_err());
        assert!(export
            .send(
                &Contracts::new(b"s".to_vec()),
                b"s",
                Duration::from_secs(60)
            )
            .is_err());
    }

    #[test]
    fn concurrent_sends_consume_one_approval_once() {
        let receiver = listener();
        let export = Arc::new(
            Export::prepare(
                &Contracts::new(b"s".to_vec()),
                &target(&receiver),
                b"one copy",
            )
            .expect("prepare"),
        );
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("review");
        let observed = receive(&receiver, ok());
        let barrier = Arc::new(Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let export = Arc::clone(&export);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    export
                        .send(
                            &Contracts::new(b"s".to_vec()),
                            b"s",
                            Duration::from_secs(60),
                        )
                        .is_ok_and(|status| status.is_some())
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
        let export = Export::prepare(
            &Contracts::new(b"s".to_vec()),
            &target(&approved),
            b"secret",
        )
        .expect("prepare");
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("review");
        assert_eq!(
            export
                .send(
                    &Contracts::new(b"s".to_vec()),
                    b"s",
                    Duration::from_secs(60)
                )
                .expect("redirect status"),
            Some(307)
        );
        assert_eq!(observed.join().expect("receiver"), b"secret");
        assert!(other.accept().is_err());
        assert!(export
            .send(
                &Contracts::new(b"s".to_vec()),
                b"s",
                Duration::from_secs(60)
            )
            .is_err());
    }

    #[test]
    fn uncertain_delivery_does_not_restore_approval() {
        let receiver = listener();
        let observed = receive(&receiver, String::new());
        let export = Export::prepare(
            &Contracts::new(b"s".to_vec()),
            &target(&receiver),
            b"secret",
        )
        .expect("prepare");
        export
            .review_from_host(&Contracts::new(b"s".to_vec()))
            .expect("review");
        assert!(export
            .send(
                &Contracts::new(b"s".to_vec()),
                b"s",
                Duration::from_secs(60)
            )
            .expect("uncertain result")
            .is_none());
        assert_eq!(observed.join().expect("receiver"), b"secret");
        assert!(export
            .send(
                &Contracts::new(b"s".to_vec()),
                b"s",
                Duration::from_secs(60)
            )
            .is_err());
        assert!(receiver.accept().is_err());
    }
}
