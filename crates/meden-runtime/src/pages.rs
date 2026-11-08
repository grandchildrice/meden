//! Host-selected documentation prefetch and immutable, offline cache snapshots.
//! Only `prefetch` performs network I/O; loading/reading a cache never does.

use std::io::{self, Read};
use std::net::IpAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

const MAX_PAGES: usize = 8;
const MAX_BODY: usize = 1_048_576;
const MAX_CACHE: u64 = 64 * 1_048_576;

/// An inert response, with no active links, scripts, cookies or refresh logic.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    url: String,
    content_type: String,
    body: String,
}

impl Page {
    /// Exact host-selected request URL; this is metadata, never a fetch trigger.
    pub fn url(&self) -> &str {
        &self.url
    }
    /// Response media type, retained as untrusted text.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
    /// Complete UTF-8 response bytes as inert, untrusted document text.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Fixed documentation available to a session. No network client is retained.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pages {
    pages: Vec<Page>,
}

impl Pages {
    /// Load a host-supplied snapshot once. All later reads use these exact bytes.
    /// A cache file is trusted configuration, not authenticated web evidence.
    pub fn load(input: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        input.take(MAX_CACHE + 1).read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CACHE {
            return Err(io::Error::other("page cache exceeds 64 MiB"));
        }
        let cache: Self = serde_json::from_slice(&bytes)?;
        validate_batch(cache.pages.iter().map(|page| page.url.as_str()))?;
        for page in &cache.pages {
            if page.body.len() > MAX_BODY || page.content_type.len() > 1024 {
                return Err(io::Error::other("cached page exceeds its size budget"));
            }
        }
        Ok(cache)
    }

    /// Fetch the complete host-selected batch, before any model decisions.
    /// GET only, empty body, no credentials/proxy/redirects/subresources. This
    /// API is deliberately not registered as a model tool or session command.
    pub fn prefetch(urls: &[String]) -> io::Result<Self> {
        validate_batch(urls.iter().map(String::as_str))?;
        let mut pages = Vec::new();
        for url in urls {
            let agent = ureq::Agent::with_parts(
                page_config(),
                DefaultConnector::default(),
                PublicResolver(DefaultResolver::default()),
            );
            pages.push(fetch_page(url, &agent)?);
        }
        Ok(Self { pages })
    }

    /// Stable local handles; selection cannot produce an external effect.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Page)> {
        self.pages.iter().enumerate()
    }

    /// A miss is final and does not resolve or fetch anything.
    pub fn get(&self, id: usize) -> Option<&Page> {
        self.pages.get(id)
    }
}

fn page_config() -> ureq::config::Config {
    ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .max_idle_connections(0)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
}

// The production caller supplies only the fixed public-only client above.
fn fetch_page(url: &str, agent: &ureq::Agent) -> io::Result<Page> {
    let mut response = agent
        .get(url)
        .header("accept", "text/plain, text/html, application/json")
        .call()
        .map_err(|_| io::Error::other("page fetch refused or failed"))?;
    if !response.status().is_success() {
        return Err(io::Error::other(
            "page fetch requires a successful response",
        ));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    if content_type.len() > 1024 {
        return Err(io::Error::other("page media type exceeds size budget"));
    }
    let mut body = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(
            u64::try_from(MAX_BODY)
                .map_err(io::Error::other)?
                .saturating_add(1),
        )
        .read_to_end(&mut body)?;
    if body.len() > MAX_BODY {
        return Err(io::Error::other("page exceeds 1 MiB"));
    }
    Ok(Page {
        url: url.to_owned(),
        content_type,
        body: String::from_utf8(body).map_err(io::Error::other)?,
    })
}

fn validate_batch<'a>(urls: impl Iterator<Item = &'a str>) -> io::Result<()> {
    let mut seen = Vec::new();
    for url in urls {
        let uri: ureq::http::Uri = url
            .parse()
            .map_err(|_| io::Error::other("invalid page URI"))?;
        if url.len() > 8192
            || url.contains('#')
            || uri.scheme_str() != Some("https")
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || uri.host().is_none_or(str::is_empty)
            || seen.len() >= MAX_PAGES
            || seen.contains(&url)
        {
            return Err(io::Error::other(
                "pages require at most eight distinct HTTPS URLs without credentials or fragments",
            ));
        }
        seen.push(url);
    }
    Ok(())
}

#[derive(Debug)]
struct PublicResolver<R>(R);

impl<R: Resolver> Resolver for PublicResolver<R> {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let addresses = self.0.resolve(uri, config, timeout)?;
        // The same checked socket addresses go directly to TcpConnector. No
        // hostname re-resolution between validation and connect; mixed public
        // and private answers fail rather than silently choosing an address.
        if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
            return Err(ureq::Error::ConnectionFailed);
        }
        Ok(addresses)
    }
}

// Conservative subset of public unicast. Deny special-use ranges, including
// NAT64, mapped IPv4, 6to4/Teredo and documentation space. See IANA special-
// purpose registries linked in docs/confidentiality.md. Local routing/TLS/DNS
// remain environmental assumptions; this is not a Lean network-stack proof.
fn public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || a == 100 && (64..=127).contains(&b)
                || a == 169 && b == 254
                || a == 172 && (16..=31).contains(&b)
                || a == 192 && (b == 168 || b == 0 && matches!(c, 0 | 2) || b == 88 && c == 99)
                || a == 198 && (matches!(b, 18 | 19) || b == 51 && c == 100)
                || a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            let [a, b, _, _, _, _, _, _] = ip.segments();
            (a & 0xe000) == 0x2000
                && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8)
                    || a == 0x2002
                    || a == 0x3fff && b < 0x1000)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn excludes_special_use_and_transition_addresses() {
        for address in [
            "0.1.2.3",
            "10.0.0.1",
            "100.64.0.1",
            "100.127.255.255",
            "127.1.2.3",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.9",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "64:ff9b:1::a00:1",
            "100::1",
            "2001::1",
            "2001:1ff::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "3fff:fff::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            assert!(!public_ip(address.parse().expect("address")), "{address}");
        }
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            "100.63.255.255",
            "100.128.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.0.1.1",
            "198.17.255.255",
            "198.20.0.0",
            "223.255.255.255",
            "2001:200::1",
            "2001:4860:4860::8888",
            "2606:4700:4700::1111",
            "3fff:1000::1",
        ] {
            assert!(public_ip(address.parse().expect("address")), "{address}");
        }
    }

    #[derive(Debug)]
    struct Answer(Vec<std::net::SocketAddr>);

    impl Resolver for Answer {
        fn resolve(
            &self,
            _: &ureq::http::Uri,
            _: &ureq::config::Config,
            _: NextTimeout,
        ) -> Result<ResolvedSocketAddrs, ureq::Error> {
            let mut answer = self.empty();
            for address in &self.0 {
                answer.push(*address);
            }
            Ok(answer)
        }
    }

    #[test]
    fn validates_the_exact_resolved_connection_addresses() {
        let uri = "https://docs.example.test/page".parse().expect("URI");
        let config = ureq::config::Config::default();
        for addresses in [
            vec![],
            vec!["127.0.0.1:443"],
            vec!["1.1.1.1:443", "10.0.0.1:443"],
            vec!["[::ffff:127.0.0.1]:443"],
        ] {
            let resolver = PublicResolver(Answer(
                addresses
                    .into_iter()
                    .map(|a| a.parse().expect("address"))
                    .collect(),
            ));
            assert!(resolver
                .resolve(&uri, &config, NextTimeout::default())
                .is_err());
        }
        let addresses = vec![
            "1.1.1.1:443".parse().expect("address"),
            "[2606:4700:4700::1111]:443".parse().expect("address"),
        ];
        let answer = PublicResolver(Answer(addresses.clone()))
            .resolve(&uri, &config, NextTimeout::default())
            .expect("public");
        assert_eq!(answer.iter().copied().collect::<Vec<_>>(), addresses);
    }

    #[test]
    fn rejects_private_connections_and_invalid_batches_without_a_socket() {
        let receiver = TcpListener::bind("127.0.0.1:0").expect("receiver");
        receiver.set_nonblocking(true).expect("nonblocking");
        let port = receiver.local_addr().expect("address").port();
        for host in ["127.0.0.1", "localhost", "[::1]", "[::ffff:127.0.0.1]"] {
            assert!(Pages::prefetch(&[format!("https://{host}:{port}/secret")]).is_err());
        }
        for url in [
            "http://1.1.1.1/",
            "https://user:password@example.test/",
            "https://example.test/#fragment",
            "file:///etc/passwd",
            "/relative",
        ] {
            assert!(validate_batch([url].into_iter()).is_err());
        }
        let urls: Vec<_> = (0..9)
            .map(|n| format!("https://example.test/{n}"))
            .collect();
        assert!(validate_batch(urls.iter().map(String::as_str)).is_err());
        assert!(
            validate_batch(["https://example.test/", "https://example.test/"].into_iter()).is_err()
        );
        assert!(receiver.accept().is_err());
    }

    #[test]
    fn cache_loading_is_bounded_inert_and_strict() {
        let original = br#"{"pages":[{"url":"https://docs.example.test/","content_type":"text/html","body":"<script>fetch('/secret')</script>"}]}"#;
        let pages = Pages::load(original.as_slice()).expect("cache");
        assert_eq!(
            pages.get(0).expect("page").body(),
            "<script>fetch('/secret')</script>"
        );
        assert!(pages.get(1).is_none());
        assert!(Pages::load(br#"{"pages":[],"approved":true}"#.as_slice()).is_err());
        let oversized = serde_json::to_vec(&serde_json::json!({"pages":[{"url":"https://docs.example.test/","content_type":"text/plain","body":"x".repeat(MAX_BODY.saturating_add(1))}]})).expect("JSON");
        assert!(Pages::load(oversized.as_slice()).is_err());
    }

    // Exercise the shared HTTP response path on an observed local receiver.
    // Only this unit-test client bypasses public-IP/HTTPS production guards.
    #[test]
    fn response_reader_is_inert_bounded_and_never_follows_redirects() {
        for (status, body, accepted) in [
            (
                200,
                "<script>fetch('/subresource')</script>".to_owned(),
                true,
            ),
            (307, String::new(), false),
            (200, "x".repeat(MAX_BODY.saturating_add(1)), false),
        ] {
            let receiver = TcpListener::bind("127.0.0.1:0").expect("receiver");
            let target = format!(
                "http://{}/exact?approved=1",
                receiver.local_addr().expect("address")
            );
            let observed = receiver.try_clone().expect("clone");
            let handle = thread::spawn(move || {
                let (stream, _) = observed.accept().expect("connect");
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .expect("timeout");
                let mut input = BufReader::new(stream);
                let mut lines = Vec::new();
                loop {
                    let mut line = String::new();
                    assert!(input.read_line(&mut line).expect("read") > 0);
                    if line == "\r\n" {
                        break;
                    }
                    lines.push(line);
                }
                let _ = write!(input.get_mut(), "HTTP/1.1 {status} Status\r\nLocation: {target}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                lines
            });
            let config = page_config();
            let result = fetch_page(
                &format!(
                    "http://{}/exact?approved=1",
                    receiver.local_addr().expect("address")
                ),
                &config.into(),
            );
            assert_eq!(result.is_ok(), accepted);
            let request = handle.join().expect("receiver");
            assert_eq!(
                request.first().map(String::as_str),
                Some("GET /exact?approved=1 HTTP/1.1\r\n")
            );
            assert!(request.iter().all(|line| !line
                .to_ascii_lowercase()
                .starts_with("authorization:")
                && !line.to_ascii_lowercase().starts_with("cookie:")));
            receiver.set_nonblocking(true).expect("nonblocking");
            assert!(receiver.accept().is_err());
        }
    }
}
