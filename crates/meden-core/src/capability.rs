//! The capability vocabulary and scoped requirements.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One kind of effect an action can have on the world.
///
/// The vocabulary is deliberately small and coarse. Finer distinctions are
/// carried by a [`Requirement`]'s `scope` string and matched by a glob.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Capability {
    /// Read files.
    #[serde(rename = "fs.read")]
    FsRead,
    /// Create, modify or delete files.
    #[serde(rename = "fs.write")]
    FsWrite,
    /// Open outbound network connections.
    #[serde(rename = "net.egress")]
    NetEgress,
    /// Accept inbound network connections.
    #[serde(rename = "net.listen")]
    NetListen,
    /// Run a shell or interpreter on a string.
    #[serde(rename = "shell.exec")]
    ShellExec,
    /// Read environment variables.
    #[serde(rename = "env.read")]
    EnvRead,
    /// Read credentials, keys, tokens.
    #[serde(rename = "secret.read")]
    SecretRead,
    /// Start other processes.
    #[serde(rename = "process.spawn")]
    ProcessSpawn,
    /// Install packages or download executables.
    #[serde(rename = "package.install")]
    PackageInstall,
    /// Read or write the clipboard.
    #[serde(rename = "clipboard")]
    Clipboard,
    /// Drive the user interface (mouse, keyboard, screen).
    #[serde(rename = "ui.control")]
    UiControl,
}

impl Capability {
    /// Every capability, in a fixed order.
    pub const ALL: [Capability; 11] = [
        Capability::FsRead,
        Capability::FsWrite,
        Capability::NetEgress,
        Capability::NetListen,
        Capability::ShellExec,
        Capability::EnvRead,
        Capability::SecretRead,
        Capability::ProcessSpawn,
        Capability::PackageInstall,
        Capability::Clipboard,
        Capability::UiControl,
    ];

    /// The canonical string name (`fs.read`, `net.egress`, ...).
    pub const fn as_str(self) -> &'static str {
        match self {
            Capability::FsRead => "fs.read",
            Capability::FsWrite => "fs.write",
            Capability::NetEgress => "net.egress",
            Capability::NetListen => "net.listen",
            Capability::ShellExec => "shell.exec",
            Capability::EnvRead => "env.read",
            Capability::SecretRead => "secret.read",
            Capability::ProcessSpawn => "process.spawn",
            Capability::PackageInstall => "package.install",
            Capability::Clipboard => "clipboard",
            Capability::UiControl => "ui.control",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string is not a capability name.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown capability: {0:?}")]
pub struct UnknownCapability(pub String);

impl FromStr for Capability {
    type Err = UnknownCapability;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Capability::ALL
            .iter()
            .copied()
            .find(|c| c.as_str() == s)
            .ok_or_else(|| UnknownCapability(s.to_owned()))
    }
}

/// One capability a skill or tool call needs, with an optional scope.
///
/// `scope` examples: `/home/user/project/**` for `fs.write`,
/// `api.github.com` for `net.egress`, `GEMINI_API_KEY` for `env.read`.
/// `None` means the whole capability, unscoped.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Requirement {
    /// The capability required.
    pub capability: Capability,
    /// Optional scope the capability is required on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Where this requirement was found (`file:line`, a metadata field, ...).
    #[serde(default)]
    pub evidence: String,
}

impl Requirement {
    /// A requirement without evidence.
    pub fn new(capability: Capability, scope: Option<&str>) -> Self {
        Self {
            capability,
            scope: scope.map(str::to_owned),
            evidence: String::new(),
        }
    }

    /// A requirement with evidence.
    pub fn with_evidence(capability: Capability, scope: Option<&str>, evidence: &str) -> Self {
        Self {
            capability,
            scope: scope.map(str::to_owned),
            evidence: evidence.to_owned(),
        }
    }

    /// `cap` or `cap[scope]`, the form used in reasons and policy files.
    pub fn label(&self) -> String {
        match &self.scope {
            Some(s) => format!("{}[{}]", self.capability, s),
            None => self.capability.to_string(),
        }
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// Parse `cap` or `cap[scope]` into a capability and optional scope.
pub fn parse_label(
    label: &str,
) -> std::result::Result<(Capability, Option<String>), UnknownCapability> {
    if let Some(rest) = label.strip_suffix(']') {
        if let Some((cap, scope)) = rest.split_once('[') {
            return Ok((cap.parse()?, Some(scope.to_owned())));
        }
    }
    Ok((label.parse()?, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for c in Capability::ALL {
            assert_eq!(c.as_str().parse::<Capability>(), Ok(c));
            let j = serde_json::to_string(&c).unwrap_or_default();
            assert_eq!(j, format!("\"{}\"", c.as_str()));
        }
        assert!("fs.exec".parse::<Capability>().is_err());
    }

    #[test]
    fn labels() {
        assert_eq!(parse_label("fs.read"), Ok((Capability::FsRead, None)));
        assert_eq!(
            parse_label("fs.read[/a/*]"),
            Ok((Capability::FsRead, Some("/a/*".into())))
        );
        assert_eq!(
            parse_label("net.egress[a[b]]"),
            Ok((Capability::NetEgress, Some("a[b]".into())))
        );
        assert!(parse_label("nope[x]").is_err());
        assert_eq!(
            Requirement::new(Capability::FsRead, Some("/a")).label(),
            "fs.read[/a]"
        );
    }
}
