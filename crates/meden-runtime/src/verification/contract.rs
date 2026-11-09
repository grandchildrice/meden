//! Host-reviewed input to the pure Rust state-transition backend.
//!
//! Deserialization is a draft, never acceptance or successful evidence. The
//! checker validates metadata and parses mathematical terms independently.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A proposed, independently checkable contract for a finite Rust transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) source: String,
    pub(super) state: String,
    pub(super) fields: Vec<String>,
    pub(super) parameters: Vec<String>,
    pub(super) function: String,
    pub(super) specification: String,
    #[serde(default)]
    pub(super) properties: Vec<Property>,
    #[serde(default)]
    pub(super) examples: Vec<Example>,
    #[serde(default)]
    pub(super) assumptions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Property {
    name: String,
    description: String,
    statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Example {
    before: Vec<u64>,
    arguments: Vec<u64>,
    after: Vec<u64>,
}

impl Contract {
    /// Model-facing draft schema; runtime validation remains authoritative.
    pub fn schema() -> serde_json::Value {
        let text = serde_json::json!({"type":"string"});
        let names = serde_json::json!({"type":"array","items":text});
        let words = serde_json::json!({"type":"array","items":{"type":"integer","minimum":0}});
        serde_json::json!({"type":"object","additionalProperties":false,
        "required":["name","description","source","state","fields","parameters","function","specification"],
        "properties":{
            "name":text,"description":text,"source":text,"state":text,
            "fields":names,"parameters":names,"function":text,"specification":text,
            "properties":{"type":"array","items":{"type":"object","additionalProperties":false,
                "required":["name","description","statement"],"properties":{"name":text,"description":text,"statement":text}}},
            "examples":{"type":"array","items":{"type":"object","additionalProperties":false,
                "required":["before","arguments","after"],"properties":{"before":words,"arguments":words,"after":words}}},
            "assumptions":names
        }})
    }

    /// Parse a bounded draft. This grants no authority to accept or verify it.
    pub fn parse(text: &str) -> io::Result<Self> {
        require(text.len() <= 65_536, "contract exceeds 64 KiB")?;
        let out: Self = serde_json::from_str(text).map_err(io::Error::other)?;
        out.validate()?;
        Ok(out)
    }

    /// Validate the supported API and resource bounds. Lean mathematical-term
    /// parsing is a separate mandatory checker step, before elaboration.
    pub fn validate(&self) -> io::Result<()> {
        require(
            !self.name.is_empty() && self.name.len() <= 256,
            "invalid contract name",
        )?;
        require(
            self.description.len() <= 4096,
            "contract description exceeds 4 KiB",
        )?;
        require(
            !self.source.is_empty()
                && self.source.len() <= 256
                && std::path::Path::new(&self.source)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
                && !self.source.contains('\\')
                && self
                    .source
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != "..")
                && !self.source.contains('\0'),
            "source must be a relative Rust file path without dot components",
        )?;
        require(
            identifier(&self.state)
                && identifier(&self.function)
                && !["Some", "None", "Option", "u64", "bool"].contains(&self.state.as_str())
                && !["Some", "None"].contains(&self.function.as_str()),
            "invalid Rust API name",
        )?;
        require(
            self.state != self.function,
            "state and function names must differ",
        )?;
        require(
            (1..=8).contains(&self.fields.len()),
            "expected one to eight state fields",
        )?;
        require(
            self.parameters.len() <= 8,
            "at most eight parameters are supported",
        )?;
        for names in [&self.fields, &self.parameters] {
            let distinct: BTreeSet<_> = names.iter().collect();
            require(
                distinct.len() == names.len(),
                "duplicate field or parameter",
            )?;
            require(
                names.iter().all(|n| identifier(n)),
                "invalid field or parameter name",
            )?;
        }
        require(
            self.parameters.iter().all(|n| {
                ![
                    "s",
                    "out",
                    "State",
                    "Nat",
                    "Bool",
                    "transition",
                    "abstractState",
                    "min",
                    "max",
                    "True",
                    "False",
                    "Some",
                    "None",
                ]
                .contains(&n.as_str())
            }),
            "parameter shadows a contract binding",
        )?;
        require(
            !self.specification.is_empty() && self.specification.len() <= 8192,
            "specification must contain at most 8 KiB",
        )?;
        require(
            self.properties.len() <= 16,
            "at most sixteen properties are supported",
        )?;
        self.validate_properties()?;
        self.validate_examples()?;
        require(
            self.assumptions.len() <= 16 && self.assumptions.iter().all(|s| s.len() <= 1024),
            "assumption budget exceeded",
        )?;
        require(self.canonical()?.len() <= 65_536, "contract exceeds 64 KiB")
    }

    fn validate_examples(&self) -> io::Result<()> {
        require(
            self.examples.len() <= 64,
            "at most sixty-four examples are supported",
        )?;
        for example in &self.examples {
            require(
                example.before.len() == self.fields.len()
                    && example.after.len() == self.fields.len()
                    && example.arguments.len() == self.parameters.len(),
                "example does not match the declared API",
            )?;
        }
        Ok(())
    }

    fn validate_properties(&self) -> io::Result<()> {
        let mut names = BTreeSet::new();
        for property in &self.properties {
            require(
                identifier(&property.name) && names.insert(&property.name),
                "invalid or duplicate property name",
            )?;
            require(
                property.description.len() <= 1024,
                "property description exceeds 1 KiB",
            )?;
            require(
                !property.statement.is_empty() && property.statement.len() <= 8192,
                "property statement must contain at most 8 KiB",
            )?;
        }
        Ok(())
    }

    /// Immutable coding-copy file to snapshot for this contract.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Exact canonical bytes used by the accepted subject and checker.
    pub fn canonical(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(io::Error::other)
    }

    /// Content identity for host review references, not an acceptance flag.
    pub fn identity(&self) -> io::Result<String> {
        Ok(format!("{:x}", Sha256::digest(self.canonical()?)))
    }

    /// Supported Rust interface, generated only from validated identifiers.
    pub fn api(&self) -> String {
        let fields = self
            .fields
            .iter()
            .map(|f| format!("pub {f}: u64"))
            .collect::<Vec<_>>()
            .join(", ");
        let parameters = self.parameters.iter().fold(String::new(), |mut out, p| {
            let _ = write!(out, ", {p}: u64");
            out
        });
        format!(
            "pub struct {} {{ {fields} }}\npub fn {}(s: {}{parameters}) -> {}",
            self.state, self.function, self.state, self.state
        )
    }

    /// Review includes the actual formal meaning. Descriptions alone are not
    /// a claim that natural-language intent has been proved equivalent.
    pub fn review_text(&self) -> String {
        let mut text = format!("{}\n\n{}\n\nSource: {}\n\nRust API:\n{}\n\nSpecification (s is the input state):\n{}\n\nAll state fields and parameters are unsigned 64-bit integers.\n", self.name, self.description, self.source, self.api(), self.specification);
        for property in &self.properties {
            let _ = write!(
                text,
                "\n{}: {}\nlet out := transition s {}; {}\n",
                property.name,
                property.description,
                self.parameters.join(" "),
                property.statement
            );
        }
        for assumption in &self.assumptions {
            let _ = write!(text, "\nAssumption: {assumption}\n");
        }
        for (index, example) in self.examples.iter().enumerate() {
            let _ = write!(
                text,
                "\nExample {index}: before={:?}; arguments={:?}; after={:?}\n",
                example.before, example.arguments, example.after
            );
        }
        text.push_str("\nProof scope: this pure transition and the listed formal properties. Not caller authentication, other state mutations, persistence, concurrency, parsers or certification. Descriptions and environmental assumptions require review; they are not proved by the checker.");
        text
    }

    pub(super) fn roots(&self) -> Vec<String> {
        std::iter::once("KernelProof.actual_refines".to_owned())
            .chain(
                self.properties
                    .iter()
                    .map(|p| format!("KernelProof.law_{}", p.name)),
            )
            .collect()
    }

    pub(super) fn specification_file(&self) -> String {
        let mut out = String::from(
            "import Lean.Elab.Tactic.Omega\nset_option linter.unusedVariables false\nnamespace KernelContract\nstructure State where\n",
        );
        for field in &self.fields {
            let _ = writeln!(out, "  «{field}» : Nat");
        }
        out.push_str("  deriving DecidableEq\n\ndef transition (s : State)");
        let arguments = self.parameters.iter().fold(String::new(), |mut out, p| {
            let _ = write!(out, " («{p}» : Nat)");
            out
        });
        out.push_str(&arguments);
        let _ = writeln!(out, " : State :=\n{}\n", self.specification);
        for property in &self.properties {
            let _ = writeln!(
                out,
                "def property_{} (s : State){arguments} (out : State) : Prop :=\n{}\n",
                property.name, property.statement
            );
            let _ = write!(out, "theorem law_{} (s : State){arguments}", property.name);
            for (index, field) in self.fields.iter().enumerate() {
                let _ = write!(
                    out,
                    " (_state_bound_{index} : s.«{field}» ≤ 18446744073709551615)"
                );
            }
            for (index, parameter) in self.parameters.iter().enumerate() {
                let _ = write!(
                    out,
                    " (_parameter_bound_{index} : «{parameter}» ≤ 18446744073709551615)"
                );
            }
            let _ = writeln!(out, " :\n    property_{} s {} (transition s {}) := by\n  simp only [property_{}, transition]\n  all_goals try simp_all\n  all_goals repeat first\n    | (split <;> try simp_all)\n    | omega\n  all_goals omega\n", property.name, self.lean_arguments(), self.lean_arguments(), property.name);
        }
        out.push_str("end KernelContract\n");
        out
    }

    pub(super) fn lean_arguments(&self) -> String {
        self.parameters
            .iter()
            .map(|p| format!("«{p}»"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn proof_file(&self) -> String {
        let abstract_fields = self
            .fields
            .iter()
            .map(|f| format!("«{f}» := s.«{f}».val"))
            .collect::<Vec<_>>()
            .join(", ");
        let declaration = format!("def abstractState (s : KernelImpl.kernel.«{}») : KernelContract.State :=\n  {{ {abstract_fields} }}", self.state);
        let native_names = self
            .parameters
            .iter()
            .enumerate()
            .map(|(i, _)| format!("_input_{i}"))
            .collect::<Vec<_>>();
        let native_arguments = native_names.join(" ");
        let arguments = native_names.iter().fold(String::new(), |mut out, p| {
            let _ = write!(out, " ({p} : U64)");
            out
        });
        let call = format!(
            "KernelImpl.kernel.«{}» _state {native_arguments}",
            self.function
        );
        let spec_arguments = native_names
            .iter()
            .map(|p| format!("{p}.val"))
            .collect::<Vec<_>>()
            .join(" ");
        let spec_call =
            format!("KernelContract.transition (abstractState _state) {spec_arguments}");
        let mut out = include_str!("../../../../verification/kernel/Check.lean")
            .replace("@@ABSTRACT_STATE@@", &declaration)
            .replace(
                "@@STATE_TYPE@@",
                &format!("KernelImpl.kernel.«{}»", self.state),
            )
            .replace("@@ARGUMENTS@@", &arguments)
            .replace("@@ARG_NAMES@@", &native_arguments)
            .replace(
                "@@ENTRY@@",
                &format!("KernelImpl.kernel.«{}»", self.function),
            )
            .replace("@@NATIVE_CALL@@", &call)
            .replace("@@SPEC_CALL@@", &spec_call);
        out.push_str("namespace KernelProof\n");
        for property in &self.properties {
            let _ = writeln!(out, "theorem law_{} (_state : KernelImpl.kernel.«{}»){arguments} :\n    ∃ _result, {call} = ok _result ∧\n      KernelContract.property_{} (abstractState _state) {spec_arguments} (abstractState _result) := by\n  obtain ⟨_result, hr, he⟩ := success _state {native_arguments}\n  refine ⟨_result, hr, ?_⟩\n  simpa only [he] using (KernelContract.law_{} (abstractState _state) {spec_arguments}", property.name, self.state, property.name, property.name);
            for field in &self.fields {
                let _ = writeln!(
                    out,
                    "    (by simpa [abstractState] using U64.le_max _state.«{field}»)"
                );
            }
            for native in &native_names {
                let _ = writeln!(out, "    (U64.le_max {native})");
            }
            out.push_str("  )\n");
        }
        out.push_str("end KernelProof\n");
        for root in self.roots() {
            let _ = writeln!(out, "#print axioms {root}");
        }
        out
    }

    pub(super) fn library_file(&self) -> String {
        if self.examples.is_empty() {
            return "pub mod kernel;\n".to_owned();
        }
        let mut out = String::from("pub mod kernel;\n#[cfg(test)]\nmod tests {\nuse super::kernel::*;\n#[test]\nfn accepted_examples() {\n");
        let read_fields = self
            .fields
            .iter()
            .map(|f| format!("out.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        for (index, example) in self.examples.iter().enumerate() {
            let fields = self
                .fields
                .iter()
                .zip(&example.before)
                .map(|(f, v)| format!("{f}: {v}u64"))
                .collect::<Vec<_>>()
                .join(", ");
            let arguments = example.arguments.iter().fold(String::new(), |mut out, v| {
                let _ = write!(out, ", {v}u64");
                out
            });
            let _ = writeln!(out, "let out = {}({} {{ {fields} }}{arguments});\nassert_eq!([{read_fields}], {:?}, \"accepted contract example {index}; before={:?}; arguments={:?}\");", self.function, self.state, example.after,example.before,example.arguments);
        }
        out.push_str("}\n}\n");
        out
    }
}

fn require(condition: bool, message: &str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

fn identifier(name: &str) -> bool {
    name.len() <= 32
        && name.bytes().next().is_some_and(|b| b.is_ascii_alphabetic())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && ![
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
            "box", "do", "final", "gen", "macro", "override", "priv", "try", "typeof", "unsized",
            "virtual", "yield",
        ]
        .contains(&name)
}
