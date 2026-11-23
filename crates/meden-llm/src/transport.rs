//! Host registrations for typed HTTP wire adapters. Discovery and metadata never
//! grant inference; the shared executor still consumes an original HttpCall.
use meden_agent::conversation::{Delta, ModelError, Reply};
use meden_runtime::plugin::{Definition, Descriptor, Role};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read, sync::Arc};

/// Frozen shared HTTP execution limits, independent of adapter selection.
pub const RESPONSE_BYTES: u64 = 16 * 1024 * 1024;
/// No retry, redirects or proxy fallback occurs after this request budget.
pub const TIMEOUT_SECONDS: u64 = 300;

/// A host-owned decoder/encoder. It receives neither an endpoint nor a key,
/// and cannot replace the shared executor's consuming processor parameters.
pub trait Wire: Send + Sync {
    /// Original implementation/configuration/schema identity.
    fn descriptor(&self) -> Result<Descriptor, ModelError>;
    /// Relative API route frozen before approval.
    fn path(&self) -> &str;
    /// Non-authority headers frozen with the method. Routing/auth headers are refused.
    fn headers(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Whether the existing read-only model-list endpoint is supported.
    fn supports_model_listing(&self) -> bool {
        false
    }
    /// Convert the original model and conversation; no HTTP operation is exposed.
    fn encode(&self, model: &str, messages: &[Value], tools: &[Value])
        -> Result<Value, ModelError>;
    /// Observe a bounded body. Only its complete decoded reply can admit calls.
    fn decode(
        &self,
        body: &mut dyn Read,
        content_type: Option<&str>,
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError>;
}

/// Backend-owned configuration is decoded once at this host registration.
pub trait Backend: Send + Sync + 'static {
    /// Original typed configuration; implementations must reject unknown keys.
    type Configuration: DeserializeOwned + Serialize + schemars::JsonSchema;
    /// Immutable prepared adapter type.
    type Client: Wire + 'static;
    /// Stable registration identifier, not an executable or endpoint.
    fn id(&self) -> &'static str;
    /// Prepare without discovery, inference or approval.
    fn prepare(&self, configuration: Self::Configuration) -> Result<Self::Client, ModelError>;
}

trait Registration: Send + Sync {
    fn prepare(&self, configuration: Value) -> Result<(Arc<dyn Wire>, Value), ModelError>;
}
struct Typed<B>(B);
impl<B: Backend> Registration for Typed<B> {
    fn prepare(&self, configuration: Value) -> Result<(Arc<dyn Wire>, Value), ModelError> {
        let decoded: B::Configuration = serde_json::from_value(configuration)
            .map_err(|_| unavailable("Invalid registered model configuration"))?;
        let configuration = json!({"value":serde_json::to_value(&decoded).map_err(|_| unavailable("Model configuration encoding failed"))?, "schema":schemars::schema_for!(B::Configuration)});
        Ok((Arc::new(self.0.prepare(decoded)?), configuration))
    }
}

/// Explicit native registrations. Model/workspace data cannot install a factory.
#[derive(Default)]
pub struct Registry {
    registrations: BTreeMap<String, Box<dyn Registration>>,
}
impl Registry {
    /// Install the built-in wire formats through the same extension interface.
    pub fn host() -> Result<Self, ModelError> {
        let mut registry = Self::default();
        registry.register(ChatCompletions)?;
        registry.register(Responses)?;
        registry.register(Claude)?;
        Ok(registry)
    }
    /// Register once; malformed/duplicate IDs and unbounded registries are refused.
    pub fn register<B: Backend>(&mut self, backend: B) -> Result<(), ModelError> {
        let id = backend.id();
        if !identifier(id) || self.registrations.contains_key(id) || self.registrations.len() >= 64
        {
            return Err(unavailable(
                "Invalid or duplicate model transport registration",
            ));
        }
        self.registrations
            .insert(id.into(), Box::new(Typed(backend)));
        Ok(())
    }
    pub(crate) fn select(&self, provider: &crate::Provider) -> Result<Selected, ModelError> {
        let (id, configuration) = provider.protocol.registration();
        let registration = self
            .registrations
            .get(id)
            .ok_or_else(|| unavailable("Model transport is not registered"))?;
        let (client, configuration) = registration.prepare(configuration)?;
        let definition = client.descriptor()?;
        if definition.id() != id
            || definition.role() != Role::Model
            || !definition
                .accepts_schema(
                    &meden_app::model::input_schema()
                        .map_err(|_| unavailable("Invalid model input schema"))?,
                )
                .map_err(|_| unavailable("Invalid model definition"))?
            || !definition
                .accepts_output_schema(
                    &meden_app::model::output_schema()
                        .map_err(|_| unavailable("Invalid model output schema"))?,
                )
                .map_err(|_| unavailable("Invalid model definition"))?
        {
            return Err(unavailable(
                "Model transport definition does not match its typed role",
            ));
        }
        let path = client.path().to_owned();
        let headers = client.headers();
        let model_listing = client.supports_model_listing();
        validate_path(&path)?;
        validate_headers(&headers)?;
        let binding = json!({"provider":provider.id,"protocol":provider.protocol,"configuration":configuration,"definition":definition,"path":path,"headers":headers,"model_listing":model_listing});
        let identity =
            serde_json::to_vec(&binding).map_err(|_| unavailable("Invalid model binding"))?;
        let selected = Selected {
            client,
            definition,
            path,
            headers,
            identity,
            binding,
            model_listing,
        };
        selected.validate()?;
        Ok(selected)
    }
}

pub(crate) struct Selected {
    client: Arc<dyn Wire>,
    pub(crate) definition: Descriptor,
    pub(crate) path: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) identity: Vec<u8>,
    pub(crate) binding: Value,
    model_listing: bool,
}
impl Selected {
    pub(crate) fn validate(&self) -> Result<(), ModelError> {
        if self.client.descriptor()? != self.definition
            || self.client.path() != self.path
            || self.client.headers() != self.headers
            || self.client.supports_model_listing() != self.model_listing
        {
            return Err(unavailable("Originally selected model transport changed"));
        }
        Ok(())
    }
    pub(crate) fn supports_model_listing(&self) -> bool {
        self.model_listing
    }
    pub(crate) fn encode(
        &self,
        model: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<Value, ModelError> {
        self.validate()?;
        self.client.encode(model, messages, tools)
    }
    pub(crate) fn decode(
        &self,
        body: &mut dyn Read,
        content_type: Option<&str>,
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        let reply = self.client.decode(body, content_type, notify)?;
        self.validate()?;
        Ok(reply)
    }
}

fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
}
fn validate_path(path: &str) -> Result<(), ModelError> {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.len() > 4096
        || path
            .bytes()
            .any(|c| matches!(c, b'?' | b'#' | 0..=32 | 127))
    {
        return Err(unavailable("Invalid registered model API path"));
    }
    Ok(())
}
fn validate_headers(headers: &[(String, String)]) -> Result<(), ModelError> {
    if headers.len() > 32 {
        return Err(unavailable("Too many registered model headers"));
    }
    let mut names = std::collections::BTreeSet::new();
    for (name, value) in headers {
        let lower = name.to_ascii_lowercase();
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || value.len() > 8192
            || value.bytes().any(|c| matches!(c, 0..=31 | 127))
            || matches!(
                lower.as_str(),
                "authorization"
                    | "proxy-authorization"
                    | "host"
                    | "content-length"
                    | "transfer-encoding"
                    | "connection"
            )
            || !names.insert(lower)
        {
            return Err(unavailable(
                "Invalid or authority-changing registered model header",
            ));
        }
    }
    Ok(())
}
pub(crate) fn unavailable(message: &str) -> ModelError {
    ModelError::Unavailable(message.into())
}

/// Standard configuration without extra parameters.
#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
/// Host-owned account selection for the Responses protocol.
#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseAccount {
    account_id: Option<String>,
}

/// Registered Chat Completions implementation.
pub struct ChatCompletions;
/// Registered Responses implementation.
pub struct Responses;
/// Registered Anthropic Messages implementation.
pub struct Claude;

/// Prepared standard format. Fields remain private and immutable.
pub struct Standard {
    format: crate::stream::Format,
    definition: Descriptor,
    path: &'static str,
    headers: Vec<(String, String)>,
}
impl Standard {
    fn new(
        id: &str,
        format: crate::stream::Format,
        path: &'static str,
        headers: Vec<(String, String)>,
    ) -> Result<Self, ModelError> {
        let implementation = [
            include_bytes!("transport.rs").as_slice(),
            include_bytes!("openai.rs").as_slice(),
            include_bytes!("responses.rs").as_slice(),
            include_bytes!("claude.rs").as_slice(),
            include_bytes!("stream.rs").as_slice(),
        ]
        .concat();
        let definition = Descriptor::new(Definition {
            id, role:Role::Model,version:"1",formats:vec![id.into()],implementation:&implementation,
            configuration:json!({"path":path,"headers":headers,"timeout_seconds":TIMEOUT_SECONDS,"response_bytes":RESPONSE_BYTES,"redirects":0,"proxy":false}),
            input:meden_app::model::input_schema().map_err(|_|unavailable("Invalid model schema"))?,
            output:meden_app::model::output_schema().map_err(|_|unavailable("Invalid model schema"))?,
        }).map_err(|_|unavailable("Invalid model definition"))?;
        Ok(Self {
            format,
            definition,
            path,
            headers,
        })
    }
}
impl Backend for ChatCompletions {
    type Configuration = Empty;
    type Client = Standard;
    fn id(&self) -> &'static str {
        "openai.chat"
    }
    fn prepare(&self, _: Empty) -> Result<Standard, ModelError> {
        Standard::new(
            self.id(),
            crate::stream::Format::Chat,
            "/chat/completions",
            Vec::new(),
        )
    }
}
impl Backend for Responses {
    type Configuration = ResponseAccount;
    type Client = Standard;
    fn id(&self) -> &'static str {
        "openai.responses"
    }
    fn prepare(&self, configuration: ResponseAccount) -> Result<Standard, ModelError> {
        let mut headers = vec![("accept".into(), "text/event-stream".into())];
        if let Some(account) = configuration.account_id {
            headers.push(("chatgpt-account-id".into(), account));
        }
        Standard::new(
            self.id(),
            crate::stream::Format::Responses,
            "/responses",
            headers,
        )
    }
}
impl Backend for Claude {
    type Configuration = Empty;
    type Client = Standard;
    fn id(&self) -> &'static str {
        "anthropic.messages"
    }
    fn prepare(&self, _: Empty) -> Result<Standard, ModelError> {
        Standard::new(
            self.id(),
            crate::stream::Format::Claude,
            "/messages",
            vec![
                ("anthropic-version".into(), "2023-06-01".into()),
                ("anthropic-beta".into(), "oauth-2025-04-20".into()),
            ],
        )
    }
}
impl Wire for Standard {
    fn descriptor(&self) -> Result<Descriptor, ModelError> {
        Ok(self.definition.clone())
    }
    fn path(&self) -> &str {
        self.path
    }
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }
    fn supports_model_listing(&self) -> bool {
        self.format == crate::stream::Format::Chat
    }
    fn encode(
        &self,
        model: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<Value, ModelError> {
        match self.format {
            crate::stream::Format::Chat => Ok(crate::openai::request_body(model, messages, tools)),
            crate::stream::Format::Responses => {
                crate::responses::request_body(model, messages, tools)
            }
            crate::stream::Format::Claude => crate::claude::request_body(model, messages, tools),
        }
    }
    fn decode(
        &self,
        body: &mut dyn Read,
        content_type: Option<&str>,
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        crate::stream::read_body(body, content_type, self.format, notify, RESPONSE_BYTES)
    }
}
