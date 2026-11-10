//! Real DNS/TCP/TLS in a disposable network namespace. Only test trust roots vary.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const TEST: &str =
    "pages::network_tests::public_https_uses_checked_dns_and_rejects_unapproved_hops";
const PORT: u16 = 28443;

fn program(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("test tool PATH"))
        .filter_map(|path| path.join(name).canonicalize().ok())
        .find(|path| {
            path.is_file()
                && ["/nix/store", "/usr", "/bin"]
                    .iter()
                    .any(|root| path.starts_with(root))
        })
        .expect("installed public namespace-test tool")
}

struct Fixture {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Fixture {
    fn start() -> Self {
        let mut child = Command::new(program("python3"))
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pages/network_fixture.py"))
            .arg(PORT.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("DNS/HTTPS fixture");
        let input = child.stdin.take().expect("fixture stdin");
        let output = BufReader::new(child.stdout.take().expect("fixture stdout"));
        Self {
            child,
            input,
            output,
        }
    }

    fn read(&mut self) -> serde_json::Value {
        let mut line = String::new();
        assert!(self.output.read_line(&mut line).expect("fixture output") > 0);
        serde_json::from_str(&line).expect("fixture JSON")
    }

    fn snapshot(&mut self) -> serde_json::Value {
        writeln!(self.input, "snapshot").expect("snapshot request");
        self.input.flush().expect("flush");
        self.read()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn public_https_uses_checked_dns_and_rejects_unapproved_hops() {
    if std::env::var_os("MEDEN_PAGE_NETWORK_CHILD").is_some() {
        exercise_network();
        return;
    }
    let scratch = tempfile::tempdir().expect("network fixture directory");
    resolver_files(scratch.path());
    // Ubuntu's default bwrap child profile denies namespace capabilities.
    // Only this trusted synthetic network lab may select a dedicated launcher.
    let bwrap = std::env::var_os("MEDEN_PAGE_NETWORK_BWRAP")
        .or_else(|| std::env::var_os("MEDEN_BWRAP"))
        .map_or_else(|| program("bwrap"), PathBuf::from);
    let mut command = Command::new(bwrap);
    command
        .args([
            "--unshare-user",
            "--unshare-net",
            "--unshare-pid",
            "--uid",
            "0",
            "--gid",
            "0",
            "--cap-add",
            "CAP_NET_ADMIN",
            "--cap-add",
            "CAP_NET_BIND_SERVICE",
            "--die-with-parent",
            "--ro-bind",
            "/",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/etc",
            "--bind",
        ])
        .arg(scratch.path())
        .arg(scratch.path());
    hide_host_services(&mut command);
    for name in ["resolv.conf", "nsswitch.conf", "hosts"] {
        command
            .arg("--ro-bind")
            .arg(scratch.path().join(name))
            .arg(format!("/etc/{name}"));
    }
    command
        .arg("--")
        .arg(program("bash"))
        .args([
            "--noprofile",
            "--norc",
            "-c",
            r#"
set -eu
"$1" link set lo up
"$1" address add 93.184.216.34/32 dev lo
"$1" address add 169.254.169.254/32 dev lo
"$1" -6 address add 2606:4700:4700::1111/128 dev lo nodad
exec "$2" --exact "$3" --nocapture
"#,
            "page-network-test",
        ])
        .arg(program("ip"))
        .arg(std::env::current_exe().expect("test executable"))
        .arg(TEST)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").expect("test PATH"))
        .env("MEDEN_PAGE_NETWORK_CHILD", scratch.path())
        .env(
            "MEDEN_PAGE_PARENT_NET",
            std::fs::read_link("/proc/self/ns/net").expect("parent net"),
        )
        .current_dir("/");
    if let Some(libraries) = std::env::var_os("LD_LIBRARY_PATH") {
        command.env("LD_LIBRARY_PATH", libraries);
    }
    for name in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"] {
        command.env(name, format!("http://127.0.0.1:{PORT}"));
    }
    command.env("NO_PROXY", "");
    command.env(
        "SSL_CERT_FILE",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pages/testdata/ca.pem"),
    );
    let output = command.output().expect("network namespace test");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let observation = scratch.path().join("observation.json");
    assert!(
        observation.is_file(),
        "child did not complete its network observations"
    );
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/verification");
    std::fs::create_dir_all(&directory).expect("observation directory");
    std::fs::copy(observation, directory.join("page-network-observation.json"))
        .expect("retain observation");
}

fn hide_host_services(command: &mut Command) {
    // libc may consult nscd over a filesystem Unix socket even with a private
    // network namespace. Hide host service directories as well as the network.
    command.args([
        "--tmpfs",
        "/run",
        "--tmpfs",
        "/var",
        "--symlink",
        "/run",
        "/var/run",
    ]);
    let profile = Path::new("/run/current-system");
    if profile.exists() {
        command
            .arg("--ro-bind")
            .arg(profile.canonicalize().expect("public Nix runtime"))
            .arg(profile);
    }
}

fn exercise_network() {
    let net = std::fs::read_link("/proc/self/ns/net").expect("child network");
    assert_ne!(
        net,
        PathBuf::from(std::env::var_os("MEDEN_PAGE_PARENT_NET").expect("parent network"))
    );
    let mut fixture = Fixture::start();
    let port =
        u16::try_from(fixture.read()["port"].as_u64().expect("fixture port")).expect("TCP port");
    assert_eq!(port, PORT);
    let certificate =
        ureq::tls::Certificate::from_pem(include_bytes!("testdata/ca.pem")).expect("test CA");
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::new_with_certs(&[certificate]))
        .build();
    assert!(!tls.disable_verification());
    let agent = page_agent(page_config_with_tls(tls));
    for host in ["docs.meden.test", "v6.meden.test"] {
        let url = format!("https://{host}:{port}/exact?host-selected=1");
        validate_batch([url.as_str()].into_iter()).expect("valid host-selected URL");
        let page = fetch_page(&url, &agent).expect("valid public DNS and TLS");
        assert_eq!(page.url(), url);
        assert_eq!(
            page.body(),
            "<script>fetch('https://subresource.meden.test/secret')</script>"
        );
    }
    for target in [
        "private.meden.test",
        "metadata.meden.test",
        "unapproved.meden.test",
    ] {
        assert!(
            fetch_page(
                &format!("https://docs.meden.test:{port}/redirect/{target}"),
                &agent
            )
            .is_err(),
            "unapproved redirect accepted: {target}"
        );
    }
    let after_redirects = fixture.snapshot();
    let dns = after_redirects["dns"].as_array().expect("DNS observations");
    assert!(
        dns.iter().all(|query| ["docs.meden.test", "v6.meden.test"]
            .contains(&query["name"].as_str().expect("hostname"))),
        "unexpected follow-up DNS: {dns:?}"
    );
    for host in [
        "private.meden.test",
        "metadata.meden.test",
        "mixed.meden.test",
        "mapped.meden.test",
    ] {
        assert!(
            fetch_page(&format!("https://{host}:{port}/refused"), &agent).is_err(),
            "nonpublic DNS admitted: {host}"
        );
    }
    let rebind = format!("https://rebind.meden.test:{port}/frozen-address");
    assert!(fetch_page(&rebind, &agent).is_ok());
    assert!(fetch_page(&rebind, &agent).is_err());
    // Actual TLS verification remains enabled: wrong name and unknown CA both
    // fail before any HTTP bytes, even though their public TCP endpoint works.
    assert!(
        fetch_page(
            &format!("https://wrong.other.test:{port}/wrong-name"),
            &agent
        )
        .is_err(),
        "wrong TLS identity admitted"
    );
    assert!(fetch_page(
        &format!("https://docs.meden.test:{port}/unknown-ca"),
        &page_agent(page_config())
    )
    .is_err());
    let denied = fixture.snapshot();
    check_observations(&denied);
    receiver_controls(port);
    let final_snapshot = fixture.snapshot();
    let controls = final_snapshot["controls"].as_array().expect("controls");
    assert_eq!(controls.len(), 3);
    assert!(controls
        .iter()
        .all(|control| control["body"] == "positive-control"));
    let directory =
        PathBuf::from(std::env::var_os("MEDEN_PAGE_NETWORK_CHILD").expect("observation directory"));
    std::fs::write(
        directory.join("observation.json"),
        serde_json::to_vec_pretty(&final_snapshot).expect("observation JSON"),
    )
    .expect("observation");
}

fn check_observations(denied: &serde_json::Value) {
    assert!(
        denied["connections"]
            .as_array()
            .expect("connections")
            .iter()
            .all(|role| role == "public4" || role == "public6"),
        "unapproved receiver connected: {}",
        denied["connections"]
    );
    assert!(denied["controls"]
        .as_array()
        .expect("private receiver")
        .is_empty());
    let requests = denied["http"].as_array().expect("HTTP observations");
    assert_eq!(requests.len(), 6);
    assert!(requests.iter().any(|request| request["role"] == "public6"));
    for request in requests {
        let text = request["request"].as_str().expect("wire request");
        assert!(text.starts_with("GET "));
        assert!(text.ends_with("\r\n\r\n"));
        assert!(!text.to_ascii_lowercase().contains("authorization:"));
        assert!(!text.to_ascii_lowercase().contains("cookie:"));
        assert!(!text.starts_with("GET /unapproved "));
        assert!(!text.starts_with("GET /wrong-name "));
        assert!(!text.starts_with("GET /unknown-ca "));
    }
    let answers: Vec<_> = denied["dns"]
        .as_array()
        .expect("DNS observations")
        .iter()
        .filter(|query| query["name"] == "rebind.meden.test" && query["type"] == 1)
        .map(|query| query["answers"].clone())
        .collect();
    assert_eq!(
        answers,
        vec![
            serde_json::json!(["93.184.216.34"]),
            serde_json::json!(["127.0.0.1"])
        ]
    );
}

fn resolver_files(directory: &Path) {
    for (name, body) in [
        (
            "resolv.conf",
            "nameserver 127.0.0.1\noptions timeout:1 attempts:1\n",
        ),
        ("nsswitch.conf", "hosts: files dns\n"),
        ("hosts", "127.0.0.1 localhost\n::1 localhost\n"),
    ] {
        std::fs::write(directory.join(name), body).expect("fixture resolver config");
    }
}

fn receiver_controls(port: u16) {
    // Prove private/metadata/IPv6 listeners really are reachable in this lab.
    for address in [
        format!("127.0.0.1:{port}"),
        format!("169.254.169.254:{port}"),
        format!("[::1]:{port}"),
    ] {
        let mut stream = TcpStream::connect(address).expect("positive receiver control");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("control timeout");
        stream
            .write_all(b"positive-control")
            .expect("control write");
        let mut reply = [0; 3];
        stream
            .read_exact(&mut reply)
            .expect("control acknowledgement");
        assert_eq!(&reply, b"ack");
    }
}
