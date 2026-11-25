//! A new native registration uses the public port and actual shared HTTP executor.
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use meden_agent::conversation::{ChatModel, Delta, ModelError, Reply};
use meden_llm::{
    transport::{Backend, Registry, Wire},
    Gateway, Protocol, Provider,
};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    workspace::SessionId,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Configuration {
    prefix: String,
}
struct Extension {
    phase: Arc<AtomicUsize>,
    role: Role,
    bad_schema: bool,
    path: &'static str,
    header: &'static str,
}
impl Extension {
    fn new(phase: &Arc<AtomicUsize>) -> Self {
        Self {
            phase: phase.clone(),
            role: Role::Model,
            bad_schema: false,
            path: "/custom",
            header: "x-custom-version",
        }
    }
}
struct Client {
    host: Extension,
    configuration: Configuration,
}
impl Backend for Extension {
    type Configuration = Configuration;
    type Client = Client;
    fn id(&self) -> &'static str {
        "test.custom"
    }
    fn prepare(&self, configuration: Configuration) -> Result<Client, ModelError> {
        Ok(Client {
            host: Self {
                phase: self.phase.clone(),
                role: self.role,
                bad_schema: self.bad_schema,
                path: self.path,
                header: self.header,
            },
            configuration,
        })
    }
}
impl Wire for Client {
    fn descriptor(&self) -> Result<Descriptor, ModelError> {
        let phase = self.host.phase.load(Ordering::SeqCst);
        Descriptor::new(Definition {
            id: "test.custom", role: self.host.role, version: "1", formats: vec!["test.custom".into()], implementation: b"custom encoder and complete JSON decoder v1",
            configuration: json!({"prefix":self.configuration.prefix,"phase": if (10..=12).contains(&phase) { 0 } else { phase }}),
            input: if self.host.bad_schema { json!({}) } else { meden_app::model::input_schema().expect("input") },
            output: meden_app::model::output_schema().expect("output"),
        }).map_err(|e| ModelError::Unavailable(e.to_string()))
    }
    fn path(&self) -> &str {
        if self.host.phase.load(Ordering::SeqCst) == 11 {
            "/changed"
        } else {
            self.host.path
        }
    }
    fn headers(&self) -> Vec<(String, String)> {
        vec![(
            self.host.header.into(),
            if self.host.phase.load(Ordering::SeqCst) == 10 {
                "2"
            } else {
                "1"
            }
            .into(),
        )]
    }
    fn supports_model_listing(&self) -> bool {
        self.host.phase.load(Ordering::SeqCst) == 12
    }
    fn encode(
        &self,
        model: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<Value, ModelError> {
        Ok(
            json!({"selected":model,"prefix":self.configuration.prefix,"dialogue":messages,"functions":tools}),
        )
    }
    fn decode(
        &self,
        body: &mut dyn Read,
        _: Option<&str>,
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        let phase = self.host.phase.load(Ordering::SeqCst);
        if phase == 2 {
            return Ok(Reply::default());
        } // Deliberately incomplete decoder.
        let mut bytes = Vec::new();
        body.read_to_end(&mut bytes)
            .map_err(|e| ModelError::InvalidResponse(e.to_string()))?;
        let reply = serde_json::from_slice(&bytes)
            .map_err(|e| ModelError::InvalidResponse(e.to_string()))?;
        notify(Delta::Text("provisional"));
        if phase == 3 {
            self.host.phase.store(4, Ordering::SeqCst);
        }
        Ok(reply)
    }
}
fn provider(listener: &TcpListener, configuration: Value) -> Provider {
    Provider {
        id: "custom-host".into(),
        label: "Custom native format".into(),
        model: "original-model".into(),
        base_url: format!("http://{}/v1", listener.local_addr().expect("address")),
        protocol: Protocol::Registered {
            id: "test.custom".into(),
            configuration,
        },
        api_key: Some("original-key".into()),
        note: String::new(),
    }
}
fn prepare(
    listener: &TcpListener,
    phase: &Arc<AtomicUsize>,
    configuration: Value,
) -> Result<Gateway, ModelError> {
    let mut registry = Registry::host()?;
    registry.register(Extension::new(phase))?;
    Gateway::prepare_with_registry(
        &SessionId::from_host_bytes(b"session"),
        provider(listener, configuration),
        &registry,
    )
}
fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let mut bytes = Vec::new();
    loop {
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).expect("request");
        assert_ne!(n, 0);
        bytes.extend_from_slice(&buf[..n]);
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..end]).expect("headers");
            let length: usize = header
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().expect("length"))
                })
                .expect("length header");
            if bytes.len().saturating_sub(end.saturating_add(4)) >= length {
                return bytes;
            }
        }
    }
}
fn serve(
    listener: TcpListener,
    body: Vec<u8>,
    declared: Option<usize>,
) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("request");
        let bytes = read_request(&mut stream);
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",declared.unwrap_or(body.len())).expect("headers");
        // Early decoder refusal may close a large response before its last byte.
        let _ = stream.write_all(&body);
        bytes
    })
}
fn reply_body() -> Vec<u8> {
    br#"{"content":"custom reply","tool_calls":[{"id":"one","function":{"name":"inspect","arguments":"{}"}}]}"#.to_vec()
}

#[test]
fn public_registration_sends_original_bound_parameters_only_after_host_approval() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listen");
    listener.set_nonblocking(true).expect("nonblocking");
    let phase = Arc::new(AtomicUsize::new(0));
    let gateway = prepare(&listener, &phase, json!({"prefix":"typed"})).expect("gateway");
    let port: &dyn meden_app::model::Model = &gateway;
    let session = SessionId::from_host_bytes(b"session");
    let caller = meden_app::model::bind(port, &session);
    assert_eq!(caller.binding().definition().id(), "test.custom");
    assert!(!gateway.supports_model_listing());
    assert!(caller.reply(&[], &[]).is_err());
    assert!(listener.accept().is_err());
    gateway.approve_from_host().expect("approve");
    assert!(
        meden_app::model::bind(port, &SessionId::from_host_bytes(b"other-session"))
            .reply(&[], &[])
            .is_err()
    );
    assert!(listener.accept().is_err());
    listener.set_nonblocking(false).expect("blocking");
    let server = serve(listener, reply_body(), None);
    let reply = caller
        .reply(
            &[json!({"role":"user","content":"original message"})],
            &[json!({"name":"inspect"})],
        )
        .expect("reply");
    assert_eq!(reply.content.as_deref(), Some("custom reply"));
    assert_eq!(reply.tool_calls.len(), 1);
    let request = String::from_utf8(server.join().expect("server")).expect("request");
    assert!(request.starts_with("POST /v1/custom HTTP/1.1\r\n"));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: bearer original-key\r\n"));
    assert!(request
        .to_ascii_lowercase()
        .contains("x-custom-version: 1\r\n"));
    let body: Value =
        serde_json::from_str(request.split_once("\r\n\r\n").expect("body").1).expect("JSON");
    assert_eq!(
        body,
        json!({"selected":"original-model","prefix":"typed","dialogue":[{"role":"user","content":"original message"}],"functions":[{"name":"inspect"}]})
    );
    gateway.revoke_from_host().expect("revoke");
    assert!(caller.reply(&[], &[]).is_err());
    assert!(gateway.approve_from_host().is_err());
}
#[test]
fn unknown_duplicate_configuration_role_schema_route_and_authority_headers_are_refused() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listen");
    listener.set_nonblocking(true).expect("nonblocking");
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::host().expect("registry");
    assert!(Gateway::prepare_with_registry(
        &SessionId::from_host_bytes(b"session"),
        provider(&listener, json!({"prefix":"x"})),
        &registry
    )
    .is_err());
    registry.register(Extension::new(&phase)).expect("register");
    assert!(registry.register(Extension::new(&phase)).is_err());
    for config in [
        json!({}),
        json!({"prefix":7}),
        json!({"prefix":"x","unknown":true}),
    ] {
        assert!(prepare(&listener, &phase, config).is_err());
    }
    for extension in [
        Extension {
            role: Role::Compiler,
            ..Extension::new(&phase)
        },
        Extension {
            bad_schema: true,
            ..Extension::new(&phase)
        },
        Extension {
            path: "//other-host",
            ..Extension::new(&phase)
        },
        Extension {
            header: "authorization",
            ..Extension::new(&phase)
        },
        Extension {
            header: "Host",
            ..Extension::new(&phase)
        },
    ] {
        let mut registry = Registry::default();
        registry.register(extension).expect("register");
        assert!(Gateway::prepare_with_registry(
            &SessionId::from_host_bytes(b"session"),
            provider(&listener, json!({"prefix":"x"})),
            &registry
        )
        .is_err());
    }
    assert!(listener.accept().is_err());
}
#[test]
fn changed_method_cannot_be_approved_or_dispatched() {
    for (approved, changed) in [false, true]
        .into_iter()
        .flat_map(|approved| [1, 10, 11, 12].map(|changed| (approved, changed)))
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listen");
        listener.set_nonblocking(true).expect("nonblocking");
        let phase = Arc::new(AtomicUsize::new(0));
        let gateway = prepare(&listener, &phase, json!({"prefix":"x"})).expect("gateway");
        if approved {
            gateway.approve_from_host().expect("approve");
        }
        phase.store(changed, Ordering::SeqCst);
        assert!(gateway.approve_from_host().is_err());
        assert!(
            meden_app::model::bind(&gateway, &SessionId::from_host_bytes(b"session"))
                .reply(&[], &[])
                .is_err()
        );
        assert!(listener.accept().is_err());
    }
}
#[test]
fn incomplete_oversized_ignored_and_changed_responses_cannot_admit_tool_calls() {
    let mut oversized = reply_body();
    oversized.resize(
        usize::try_from(meden_llm::transport::RESPONSE_BYTES.saturating_add(1))
            .expect("body limit"),
        b' ',
    );
    for (phase_value, body, declared) in [
        (0, reply_body(), Some(reply_body().len() + 20)),
        (2, reply_body(), None),
        (0, oversized, None),
        (3, reply_body(), None),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listen");
        let phase = Arc::new(AtomicUsize::new(phase_value));
        let gateway = prepare(&listener, &phase, json!({"prefix":"x"})).expect("gateway");
        gateway.approve_from_host().expect("approve");
        let server = serve(listener, body, declared);
        assert!(
            meden_app::model::bind(&gateway, &SessionId::from_host_bytes(b"session"))
                .reply(&[], &[])
                .is_err()
        );
        server.join().expect("server");
    }
}
