//! Offline Z3 QF_BV checks of data-only finite transitions. An independent finite
//! evaluator must agree before a bounded result can become a checked artifact.
use super::{
    artifact::{Checked, Class, Inputs, Report},
    Failure, FailureKind,
};
use crate::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    io,
    path::{Path, PathBuf},
    time::Duration,
};
mod terms;
pub use terms::{Model, Predicate, Word};

/// Exact backend-owned data schema. It cannot contain executable SMT text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Format {
    /// Version one of finite unsigned bit-vector transition contracts.
    #[serde(rename = "finite-bitvectors-v1")]
    FiniteBitvectors,
}
/// A named safety property over one original input and simultaneous output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Property {
    /// Stable short label.
    pub name: String,
    /// Reviewed intended behavior.
    pub description: String,
    /// Actual finite unsigned predicate checked by the backend.
    pub statement: Predicate,
}
/// Bounded proposal data, never authority to check or accept anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// Unambiguous host-registered contract format.
    pub format: Format,
    /// Reviewed title.
    pub name: String,
    /// Reviewed natural-language intent, not a proved equivalence claim.
    pub description: String,
    /// Workspace-relative JSON transition file.
    pub source_path: String,
    /// All fields and parameters have this unsigned width (1 through 8).
    #[schemars(range(min = 1, max = 8))]
    pub width: u8,
    /// Ordered simultaneous state fields.
    #[schemars(length(min = 1, max = 4))]
    pub fields: Vec<String>,
    /// Ordered finite transition arguments.
    #[schemars(length(max = 4))]
    pub parameters: Vec<String>,
    /// Nonempty exact property set.
    #[schemars(length(min = 1, max = 16))]
    pub properties: Vec<Property>,
    /// Environmental assumptions for host review; no hidden logical premises.
    #[serde(default)]
    #[schemars(length(max = 16))]
    pub assumptions: Vec<String>,
}
fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
fn require(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
fn hash(bytes: &[u8]) -> String {
    super::artifact::hash(bytes)
}

impl Contract {
    /// Model-facing shape generated from the actual backend-owned data types.
    pub fn schema() -> io::Result<serde_json::Value> {
        serde_json::to_value(schemars::schema_for!(Self)).map_err(io::Error::other)
    }
    /// Decode only the bounded data schema; no tool is run.
    pub fn parse(text: &str) -> io::Result<Self> {
        require(text.len() <= 65_536, "Finite contract exceeds 64 KiB")?;
        let value: Self = serde_json::from_str(text)?;
        value.validate()?;
        Ok(value)
    }
    /// Check identifiers, finite scope, expression typing and computation bounds.
    pub fn validate(&self) -> io::Result<()> {
        require(
            !self.name.is_empty() && self.name.len() <= 256 && self.description.len() <= 4096,
            "Invalid finite contract metadata",
        )?;
        require(
            !self.source_path.is_empty()
                && self.source_path.len() <= 128
                && Path::new(&self.source_path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                && self.source_path.split('/').all(|p| {
                    !p.is_empty()
                        && p != "."
                        && p != ".."
                        && p.bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
                }),
            "Finite source must be a normalized relative .json path",
        )?;
        require(
            (1..=8).contains(&self.width)
                && !self.fields.is_empty()
                && self.fields.len() <= 4
                && self.parameters.len() <= 4,
            "Finite words require width 1..8, 1..4 fields and 0..4 parameters",
        )?;
        for names in [&self.fields, &self.parameters] {
            require(
                names.iter().all(|s| name(s))
                    && names.iter().collect::<BTreeSet<_>>().len() == names.len(),
                "Finite field/parameter labels must be unique ASCII identifiers",
            )?;
        }
        require(self.total_bits()<=16,"Finite verification supports at most 16 total input bits; no partial domain is checked")?;
        require(
            !self.properties.is_empty()
                && self.properties.len() <= 16
                && self
                    .properties
                    .iter()
                    .map(|p| &p.name)
                    .collect::<BTreeSet<_>>()
                    .len()
                    == self.properties.len(),
            "Declare 1..16 uniquely named finite properties",
        )?;
        require(
            self.assumptions.len() <= 16 && self.assumptions.iter().all(|s| s.len() <= 1024),
            "Finite assumptions exceed bounds",
        )?;
        let mut nodes = 0;
        for p in &self.properties {
            require(
                name(&p.name) && p.description.len() <= 1024,
                "Invalid finite property metadata",
            )?;
            p.statement.validate(
                &self.fields,
                &self.parameters,
                self.width,
                true,
                0,
                &mut nodes,
            )?;
        }
        require(
            u64::try_from(nodes)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(self.cases()?))
                <= 4_194_304,
            "Finite property/domain work exceeds the complete-check budget",
        )
    }
    fn total_bits(&self) -> usize {
        self.fields
            .len()
            .saturating_add(self.parameters.len())
            .saturating_mul(usize::from(self.width))
    }
    fn cases(&self) -> io::Result<u32> {
        1u32.checked_shl(u32::try_from(self.total_bits()).map_err(io::Error::other)?)
            .ok_or_else(|| invalid("Invalid finite domain"))
    }
    /// Original canonical property and scope bytes used for host review.
    pub fn canonical(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(io::Error::other)
    }
    /// Frozen checker recipe, schema and finite settings, without model input code.
    pub fn checker_id(&self) -> io::Result<String> {
        Ok(hash(&serde_json::to_vec(&(
            self,
            include_str!("mod.rs"),
            include_str!("terms.rs"),
            "QF_BV/unsigned/one-step/seed=0/timeout=10s",
        ))?))
    }
    /// Exact finite claim; it is not an unbounded-time or native-code proof.
    pub fn scope(&self) -> String {
        format!("Bounded one-step check of every input in the {}-bit unsigned domain ({} total input bits, {} assignments). Addition/subtraction wrap modulo 2^{}. No reachability, unbounded time, Rust correspondence or whole-application claim.",self.width,self.total_bits(),self.cases().unwrap_or(0),self.width)
    }
    /// Full unambiguous host review, independent of executable source.
    pub fn review_text(&self) -> String {
        let mut text=format!("{}\n\n{}\n\nSource: {}\n\nEvidence: bounded finite bit-vector checking\n{}\n\nState fields: {}\nParameters: {}\nAll output assignments are simultaneous. Every original state and argument is checked, not just sampled or reachable states.\n",self.name,self.description,self.source_path,self.scope(),self.fields.join(", "),self.parameters.join(", "));
        for p in &self.properties {
            let _ = writeln!(
                text,
                "\n{}: {}\n{}",
                p.name,
                p.description,
                p.statement.text()
            );
        }
        for assumption in &self.assumptions {
            let _ = writeln!(text, "\nReviewed environmental assumption: {assumption}");
        }
        text
    }
    fn model(&self, source: &str) -> io::Result<Model> {
        require(source.len() <= 65_536, "Finite source exceeds 64 KiB")?;
        let model: Model = serde_json::from_str(source)?;
        require(
            model.version == 1
                && model.transition.keys().collect::<BTreeSet<_>>() == self.fields.iter().collect(),
            "Finite transition must assign exactly every declared field",
        )?;
        let mut nodes = 0;
        for expression in model.transition.values() {
            expression.validate(
                &self.fields,
                &self.parameters,
                self.width,
                false,
                0,
                &mut nodes,
            )?;
        }
        require(
            u64::try_from(nodes)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(self.cases()?))
                <= 4_194_304,
            "Finite source/domain work exceeds the complete-check budget",
        )?;
        Ok(model)
    }
    fn program(&self, model: &Model) -> io::Result<String> {
        let width = self.width;
        let mut text=String::from("(set-option :print-success false)\n(set-option :timeout 10000)\n(set-option :smt.random_seed 0)\n(set-logic QF_BV)\n");
        for (prefix, names) in [("s", &self.fields), ("p", &self.parameters)] {
            for i in 0..names.len() {
                let _ = writeln!(text, "(declare-fun {prefix}{i} () (_ BitVec {width}))");
            }
        }
        for (i, field) in self.fields.iter().enumerate() {
            let expression = model
                .transition
                .get(field)
                .ok_or_else(|| invalid("Missing finite assignment"))?;
            let _ = writeln!(
                text,
                "(define-fun o{i} () (_ BitVec {width}) {})",
                expression.smt(&self.fields, &self.parameters, width)?
            );
        }
        let properties = self
            .properties
            .iter()
            .map(|p| p.statement.smt(&self.fields, &self.parameters, width))
            .collect::<io::Result<Vec<_>>>()?;
        let _ = writeln!(
            text,
            "(assert (not (and {})))\n(check-sat)\n(exit)",
            properties.join(" ")
        );
        require(
            text.len() <= 1_048_576,
            "Generated finite query exceeds 1 MiB",
        )?;
        Ok(text)
    }
}

/// Counterexample independently evaluated from the finite source, not solver prose.
#[derive(Debug, Serialize)]
pub struct Counterexample {
    /// Original ordered state.
    pub before: Vec<u16>,
    /// Original ordered arguments.
    pub arguments: Vec<u16>,
    /// Simultaneous calculated output.
    pub after: Vec<u16>,
    /// Exact failed accepted property.
    pub property: String,
}
fn enumerate(
    contract: &Contract,
    model: &Model,
    control: &Control,
) -> io::Result<Option<Counterexample>> {
    let mask = 1u32
        .checked_shl(u32::from(contract.width))
        .and_then(|n| n.checked_sub(1))
        .ok_or_else(|| invalid("Invalid finite width"))?;
    for assignment in 0..contract.cases()? {
        if assignment % 256 == 0 {
            control.with_active(|| Ok(()))?;
        }
        let values = (0..contract
            .fields
            .len()
            .saturating_add(contract.parameters.len()))
            .map(|i| {
                let shift = u32::try_from(i.saturating_mul(usize::from(contract.width)))
                    .map_err(io::Error::other)?;
                u16::try_from(
                    assignment
                        .checked_shr(shift)
                        .ok_or_else(|| invalid("Invalid finite input shift"))?
                        & mask,
                )
                .map_err(io::Error::other)
            })
            .collect::<io::Result<Vec<_>>>()?;
        let before = values
            .get(..contract.fields.len())
            .ok_or_else(|| invalid("Finite state vector mismatch"))?;
        let arguments = values
            .get(contract.fields.len()..)
            .ok_or_else(|| invalid("Finite argument vector mismatch"))?;
        let mut context = terms::Context {
            fields: &contract.fields,
            parameters: &contract.parameters,
            width: contract.width,
            before,
            arguments,
            after: &[],
        };
        let after = contract
            .fields
            .iter()
            .map(|field| {
                model
                    .transition
                    .get(field)
                    .ok_or_else(|| invalid("Missing finite assignment"))?
                    .eval(&context)
            })
            .collect::<io::Result<Vec<_>>>()?;
        context.after = &after;
        for property in &contract.properties {
            if !property.statement.eval(&context)? {
                return Ok(Some(Counterexample {
                    before: before.to_vec(),
                    arguments: arguments.to_vec(),
                    after,
                    property: property.name.clone(),
                }));
            }
        }
    }
    Ok(None)
}

/// Frozen one-use Z3 request over an original source and selected executable.
pub struct Prepared {
    contract: Contract,
    source: String,
    model: Model,
    program: String,
    inputs: Inputs,
    executable: PathBuf,
    objects: Vec<PathBuf>,
    worker: PathBuf,
    scratch: tempfile::TempDir,
    resources: tempfile::TempDir,
}
fn active(control: &Control) -> Result<(), Failure> {
    control.with_active(|| Ok(())).map_err(|error| {
        Failure::new(
            if error.kind() == io::ErrorKind::Interrupted {
                FailureKind::Inconclusive
            } else {
                FailureKind::Unavailable
            },
            error,
        )
    })
}

/// Prepare only a host-selected public Z3 executable. No project path selects it.
pub fn prepare(
    source: &str,
    contract: &Contract,
    worker: &Path,
    control: &Control,
) -> Result<Prepared, Failure> {
    contract
        .validate()
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    let model = contract
        .model(source)
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    let program = contract.program(&model).map_err(Failure::unavailable)?;
    active(control)?;
    let executable = crate::sandbox::Sandbox::public_tool("z3").ok_or_else(|| {
        Failure::unavailable(
            "Public Z3 is not installed; prepare Z3 on the host before selecting this backend",
        )
    })?;
    let bytes = std::fs::read(&executable)?;
    require(
        bytes.starts_with(b"\x7fELF"),
        "Z3 must be a prepared public ELF executable, not a wrapper",
    )?;
    let objects = crate::sandbox::Sandbox::tool_objects(&executable)?;
    let resources_hash = resources(&objects)?;
    let worker = worker.canonicalize()?;
    let inputs = Inputs {
        source_sha256: hash(source.as_bytes()),
        bundle_sha256: resources_hash,
        checker_sha256: contract.checker_id()?,
        worker_sha256: hash(&std::fs::read(&worker)?),
    };
    Ok(Prepared {
        contract: contract.clone(),
        source: source.into(),
        model,
        program,
        inputs,
        executable,
        objects,
        worker,
        scratch: tempfile::tempdir()?,
        resources: tempfile::tempdir()?,
    })
}
impl Prepared {
    /// Exact input/resource/checker binding issued to the existing job controller.
    pub fn inputs(&self) -> &Inputs {
        &self.inputs
    }
    /// Dispatch once; raw-zero complete output and independent agreement are mandatory.
    pub fn check(self, control: &Control) -> Result<Checked, Failure> {
        let _timing = crate::metrics::Span::start("verification.z3");
        let mut command = crate::sandbox::Sandbox::adapter_command(
            self.scratch.path(),
            &self.worker,
            &self.executable,
            self.resources.path(),
            &[],
            &["-in", "-smt2"],
        )?;
        let output = process::run_with_input(
            &mut command,
            self.program.as_bytes(),
            &Limits {
                timeout: Duration::from_secs(15),
                capture_bytes: 65_536,
                output_bytes: 131_072,
            },
            control,
        )?;
        if !matches!(output.termination,Termination::Exited(code) if code.success())
            || output.truncated
        {
            return Err(Failure::new(
                if output.truncated || !matches!(output.termination, Termination::Exited(_)) {
                    FailureKind::Inconclusive
                } else {
                    FailureKind::Unavailable
                },
                format!(
                    "Z3 did not complete: {:?}: {}",
                    output.termination, output.stderr
                ),
            ));
        }
        let verdict = verdict(&output.stdout)?;
        let counterexample = enumerate(&self.contract, &self.model, control).map_err(|error| {
            Failure::new(
                if error.kind() == io::ErrorKind::Interrupted {
                    FailureKind::Inconclusive
                } else {
                    FailureKind::Unavailable
                },
                error,
            )
        })?;
        admit(verdict, counterexample.as_ref())?;
        if resources(&self.objects)? != self.inputs.bundle_sha256
            || hash(&std::fs::read(&self.worker)?) != self.inputs.worker_sha256
        {
            return Err(Failure::unavailable(
                "Z3 or worker resources changed while checking",
            ));
        }
        active(control)?;
        let files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
            ("source.json".into(), self.source.into_bytes()),
            ("contract.json".into(), self.contract.canonical()?),
            ("query.smt2".into(), self.program.into_bytes()),
            ("solver.txt".into(), output.stdout.into_bytes()),
        ]);
        let hashes: BTreeMap<_, _> = files.iter().map(|(n, b)| (n.clone(), hash(b))).collect();
        let report=Report{contract:serde_json::to_value(&self.contract).map_err(Failure::unavailable)?,contract_sha256:hash(&self.contract.canonical()?),backend:"z3.finite.v1".into(),class:Class::Bounded,
            source_sha256:self.inputs.source_sha256,source_file:"source.json".into(),bundle_sha256:self.inputs.bundle_sha256,checker_sha256:self.inputs.checker_sha256,worker_sha256:self.inputs.worker_sha256,
            artifact_sha256:hash(&serde_json::to_vec(&hashes).map_err(Failure::unavailable)?),files:hashes,audit:vec![format!("Z3 QF_BV: unsat; independent complete enumeration: {} assignments; all {} properties hold",self.contract.cases()?,self.contract.properties.len())],scope:self.contract.scope()};
        Ok(Checked { report, files })
    }
}
fn resources(objects: &[PathBuf]) -> io::Result<String> {
    let digests = objects
        .iter()
        .map(|p| Ok((p, hash(&std::fs::read(p)?))))
        .collect::<io::Result<Vec<_>>>()?;
    Ok(hash(&serde_json::to_vec(&digests)?))
}
enum Verdict {
    Sat,
    Unsat,
}
fn verdict(stdout: &str) -> Result<Verdict, Failure> {
    match stdout.trim() {
        "sat" => Ok(Verdict::Sat),
        "unsat" => Ok(Verdict::Unsat),
        "unknown" => Err(Failure::new(
            FailureKind::Inconclusive,
            "Z3 returned unknown; no bounded result admitted",
        )),
        _ => Err(Failure::unavailable(format!(
            "Z3 returned a malformed or incomplete result; no result admitted: {:?}",
            stdout.chars().take(2048).collect::<String>()
        ))),
    }
}
fn admit(verdict: Verdict, counterexample: Option<&Counterexample>) -> Result<(), Failure> {
    match (verdict, counterexample) {
        (Verdict::Unsat, None) => Ok(()),
        (Verdict::Sat, Some(witness)) => Err(Failure::new(
            FailureKind::Unverified,
            format!(
                "Finite property failed; independently evaluated counterexample: {}",
                serde_json::to_string(witness).map_err(Failure::unavailable)?
            ),
        )),
        _ => Err(Failure::unavailable(
            "Z3 disagrees with the independent finite evaluator; no result admitted",
        )),
    }
}

#[cfg(test)]
mod tests;
