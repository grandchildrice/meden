//! Separate loopback origin serving only a frozen static bundle, never workspace I/O.
use super::{assets::Bundle, header, lock};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fmt::Write as _;
use std::io::Read;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

struct Current {
    authority: String,
    bundle: Bundle,
    errors: Vec<String>,
}
#[derive(Clone, Serialize)]
pub(super) struct Summary {
    pub root: String,
    pub entry: String,
    pub revision: String,
    pub url: String,
    pub file_count: usize,
    pub byte_count: usize,
    pub errors: Vec<String>,
}
pub(super) struct Preview {
    server: Server,
    port: u16,
    parent_origin: String,
    current: Mutex<Option<Current>>,
}
impl Preview {
    pub fn new(parent_origin: &str) -> Result<Self> {
        let server = Server::http((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let port = server
            .server_addr()
            .to_ip()
            .context("preview listener unavailable")?
            .port();
        Ok(Self {
            server,
            port,
            parent_origin: parent_origin.into(),
            current: Mutex::new(None),
        })
    }
    pub fn frame_source(&self) -> String {
        format!("http://*.localhost:{}", self.port)
    }
    pub fn clear(&self) {
        *lock(&self.current) = None;
    }
    pub fn capture(
        &self,
        workspace: &meden_runtime::workspace::Workspace,
        control: &meden_runtime::process::Control,
        root: &str,
        entry: &str,
    ) -> Result<()> {
        self.clear();
        let bundle = super::assets::capture(workspace, control, root, entry)?;
        control.with_active(|| self.install(bundle).map_err(std::io::Error::other))?;
        Ok(())
    }
    pub fn install(&self, bundle: Bundle) -> Result<()> {
        let mut token = [0u8; 24];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut token)?;
        let token = token.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        });
        let authority = format!("{token}.localhost:{}", self.port);
        *lock(&self.current) = Some(Current {
            authority,
            bundle,
            errors: Vec::new(),
        });
        Ok(())
    }
    pub fn summary(&self) -> Option<Summary> {
        lock(&self.current).as_ref().map(|current| Summary {
            root: current.bundle.root.clone(),
            entry: current.bundle.entry.clone(),
            revision: current.bundle.revision.clone(),
            url: format!(
                "http://{}/{}",
                current.authority,
                current
                    .bundle
                    .entry
                    .split('/')
                    .map(|s| percent_encoding::utf8_percent_encode(
                        s,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                    .to_string())
                    .collect::<Vec<_>>()
                    .join("/")
            ),
            file_count: current.bundle.files.len(),
            byte_count: current.bundle.byte_count,
            errors: current.errors.clone(),
        })
    }
    pub fn listen(&self, stopping: &AtomicBool) {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    while !stopping.load(Ordering::SeqCst) {
                        match self
                            .server
                            .recv_timeout(std::time::Duration::from_millis(200))
                        {
                            Ok(Some(request)) => self.handle(request),
                            Ok(None) => {}
                            Err(_) => break,
                        }
                    }
                });
            }
        });
    }
    fn handle(&self, request: Request) {
        let response = {
            let mut current = lock(&self.current);
            match current.as_mut() {
                Some(current) if header(&request, "Host") == Some(current.authority.as_str()) => {
                    let origin = format!("http://{}", current.authority);
                    let path = request
                        .url()
                        .split('?')
                        .next()
                        .unwrap_or("")
                        .trim_start_matches('/');
                    let path = percent_encoding::percent_decode_str(path)
                        .decode_utf8()
                        .ok();
                    let asset = path
                        .as_ref()
                        .and_then(|name| current.bundle.files.get(name.as_ref()));
                    if request.method() != &Method::Get {
                        (
                            405,
                            "text/plain",
                            Arc::<[u8]>::from(&b"GET only"[..]),
                            origin,
                        )
                    } else if let Some(asset) = asset {
                        (200, asset.mime, Arc::clone(&asset.bytes), origin)
                    } else {
                        if current.errors.len() < 20 {
                            current.errors.push(format!(
                                "Missing preview asset: {}",
                                path.as_deref()
                                    .unwrap_or("invalid URL")
                                    .chars()
                                    .take(500)
                                    .collect::<String>()
                            ));
                        }
                        (
                            404,
                            "text/plain",
                            Arc::<[u8]>::from(&b"File is not in this frozen preview"[..]),
                            origin,
                        )
                    }
                }
                _ => (
                    403,
                    "text/plain",
                    Arc::<[u8]>::from(&b"Preview unavailable; reread files in Meden"[..]),
                    "http://invalid.localhost".into(),
                ),
            }
        };
        let (status, mime, bytes, origin) = response;
        let mut response = Response::from_data(bytes.to_vec()).with_status_code(StatusCode(status));
        for header in self.headers(mime, &origin) {
            response.add_header(header);
        }
        let _ = request.respond(response);
    }
    fn headers(&self, mime: &str, origin: &str) -> Vec<Header> {
        let policy = format!("default-src 'none'; script-src {origin} 'unsafe-inline' 'wasm-unsafe-eval'; style-src {origin} 'unsafe-inline'; img-src {origin} data:; font-src {origin}; connect-src {origin}; media-src {origin}; worker-src 'none'; frame-src 'none'; object-src 'none'; base-uri {origin}; form-action 'none'; frame-ancestors {}; sandbox allow-scripts allow-forms", self.parent_origin);
        [
            ("Content-Type", mime),
            ("Cache-Control", "no-store"),
            ("X-Content-Type-Options", "nosniff"),
            ("Referrer-Policy", "no-referrer"),
            ("Access-Control-Allow-Origin", "null"),
            ("Cross-Origin-Resource-Policy", "cross-origin"),
            ("X-DNS-Prefetch-Control", "off"),
            ("Content-Security-Policy", &policy),
        ]
        .into_iter()
        .filter_map(|(name, value)| Header::from_bytes(name, value).ok())
        .collect()
    }
}
