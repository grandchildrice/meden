//! Offline Z3 QF_BV checks of data-only finite transitions. An independent finite
//! evaluator must agree before a bounded result can become a checked artifact.
use super::{
    artifact::{Checked, Class, Inputs, Report},
    Failure, FailureKind,
};
use crate::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io,
    path::{Path, PathBuf},
    time::Duration,
};
mod terms;
pub use terms::{Model, Predicate, Word};

mod data;
use data::{invalid, require};
pub use data::{Contract, Format, Property};
fn hash(bytes: &[u8]) -> String {
    super::artifact::hash(bytes)
}

/// Identity of the data decoder and finite term validator; does not select or run Z3.
pub fn decoding_identity() -> String {
    data::identity()
}

impl Contract {
    /// Frozen checker recipe, schema and finite settings, without model input code.
    pub fn checker_id(&self) -> io::Result<String> {
        Ok(hash(&serde_json::to_vec(&(
            self,
            descriptor()?.identity(),
        ))?))
    }
    fn program(&self, model: &Model) -> io::Result<String> {
        let width = self.width;
        let mut text=format!("(set-option :print-success false)\n(set-option :timeout {SOLVER_TIMEOUT_MS})\n(set-option :smt.random_seed {SEED})\n(set-logic QF_BV)\n");
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Counterexample {
    /// Original ordered state.
    #[schemars(length(max = 4))]
    pub before: Vec<u16>,
    /// Original ordered arguments.
    #[schemars(length(max = 4))]
    pub arguments: Vec<u16>,
    /// Simultaneous calculated output.
    #[schemars(length(max = 4))]
    pub after: Vec<u16>,
    /// Exact failed original property.
    #[schemars(length(min = 1, max = 32))]
    pub property: String,
}
/// Read-only solver/evaluator agreement. It cannot supply checked files, a job
/// completion or a verification receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    /// Complete finite enumeration and Z3 agree that this query has no witness.
    NotFound,
    /// A concrete original-domain witness independently calculated from source.
    Counterexample {
        /// Independently evaluated original-domain values and failed property.
        witness: Counterexample,
    },
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
    registration: crate::plugin::Descriptor,
    contract: Contract,
    source: String,
    model: Model,
    program: String,
    inputs: Inputs,
    executable: PathBuf,
    objects: Vec<PathBuf>,
    worker: PathBuf,
    limits: Limits,
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
    active(control)?;
    contract
        .validate()
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    let model = contract
        .decode_model(source)
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    let program = contract.program(&model).map_err(Failure::unavailable)?;
    active(control)?;
    let (executable, objects, worker, resources_hash) = selected_resources(worker)?;
    let registration = descriptor()?;
    let inputs = Inputs {
        registration_sha256: registration.identity().into(),
        source_sha256: hash(source.as_bytes()),
        bundle_sha256: resources_hash,
        checker_sha256: contract.checker_id()?,
        worker_sha256: hash(&std::fs::read(&worker)?),
    };
    Ok(Prepared {
        registration,
        contract: contract.clone(),
        source: source.into(),
        model,
        program,
        inputs,
        executable,
        objects,
        worker,
        limits: limits(),
        scratch: tempfile::tempdir()?,
        resources: tempfile::tempdir()?,
    })
}
fn selected_resources(worker: &Path) -> Result<(PathBuf, Vec<PathBuf>, PathBuf, String), Failure> {
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

    Ok((executable, objects, worker, resources_hash))
}
/// Exact current host-selected solver/worker binding for observational freshness.
/// Reads only resources; dispatch still requires a newly prepared consuming call.
pub fn resource_identity(worker: &Path) -> Result<String, Failure> {
    let (executable, _, worker, resources_hash) = selected_resources(worker)?;
    Ok(hash(
        &serde_json::to_vec(&(
            descriptor()?.identity(),
            executable,
            resources_hash,
            &worker,
            hash(&std::fs::read(&worker)?),
        ))
        .map_err(Failure::unavailable)?,
    ))
}

impl Prepared {
    /// Exact input/resource/checker binding issued to the existing job controller.
    pub fn inputs(&self) -> &Inputs {
        &self.inputs
    }
    /// Dispatch once; raw-zero complete output and independent agreement are mandatory.
    pub fn check(self, control: &Control) -> Result<Checked, Failure> {
        self.check_observed(control, None)
    }
    fn check_observed(
        self,
        control: &Control,
        observer: Option<&dyn Fn(&process::OutputProgress)>,
    ) -> Result<Checked, Failure> {
        let _timing = crate::metrics::Span::start("verification.z3");
        let output = self.execute(control, observer)?;
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
        self.current_resources()?;
        active(control)?;
        let files: BTreeMap<String, Vec<u8>> = BTreeMap::from([
            ("source.json".into(), self.source.into_bytes()),
            ("contract.json".into(), self.contract.canonical()?),
            ("query.smt2".into(), self.program.into_bytes()),
            ("solver.txt".into(), output.stdout.into_bytes()),
        ]);
        let hashes: BTreeMap<_, _> = files.iter().map(|(n, b)| (n.clone(), hash(b))).collect();
        let report=Report{contract:serde_json::to_value(&self.contract).map_err(Failure::unavailable)?,contract_sha256:hash(&self.contract.canonical()?),registration:self.registration,class:Class::Bounded,
            source_sha256:self.inputs.source_sha256,source_file:"source.json".into(),bundle_sha256:self.inputs.bundle_sha256,checker_sha256:self.inputs.checker_sha256,worker_sha256:self.inputs.worker_sha256,
            artifact_sha256:hash(&serde_json::to_vec(&hashes).map_err(Failure::unavailable)?),files:hashes,audit:vec![format!("Z3 QF_BV: unsat; independent complete enumeration: {} assignments; all {} properties hold",self.contract.cases()?,self.contract.properties.len())],scope:self.contract.scope()};
        Ok(Checked { report, files })
    }
    /// Consume one prepared observational search; no checked artifact is returned.
    pub fn observe(
        self,
        control: &Control,
        observer: Option<&dyn Fn(&process::OutputProgress)>,
    ) -> Result<Observation, Failure> {
        let _timing = crate::metrics::Span::start("analysis.z3.search");
        let output = self.execute(control, observer)?;
        let verdict = verdict(&output.stdout)?;
        let witness = enumerate(&self.contract, &self.model, control).map_err(|error| {
            Failure::new(
                if error.kind() == io::ErrorKind::Interrupted {
                    FailureKind::Inconclusive
                } else {
                    FailureKind::Unavailable
                },
                error,
            )
        })?;
        let observation =
            match (verdict, witness) {
                (Verdict::Unsat, None) => Observation::NotFound,
                (Verdict::Sat, Some(witness)) => Observation::Counterexample { witness },
                _ => return Err(Failure::unavailable(
                    "Z3 disagrees with the independent finite evaluator; no observation admitted",
                )),
            };
        self.current_resources()?;
        active(control)?;
        Ok(observation)
    }
    fn execute(
        &self,
        control: &Control,
        observer: Option<&dyn Fn(&process::OutputProgress)>,
    ) -> Result<process::Output, Failure> {
        active(control)?;
        self.current_resources()?;
        let mut command = crate::sandbox::Sandbox::adapter_command(
            self.scratch.path(),
            &self.worker,
            &self.executable,
            self.resources.path(),
            &[],
            &["-in", "-smt2"],
        )?;
        let output = process::run_with_input_observed(
            &mut command,
            self.program.as_bytes(),
            &self.limits,
            control,
            observer,
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
                    "Z3 did not complete: {:?}\nstdout: {}\nstderr: {}",
                    output.termination,
                    output.stdout.chars().take(4096).collect::<String>(),
                    output.stderr.chars().take(4096).collect::<String>()
                ),
            ));
        }
        Ok(output)
    }
    fn current_resources(&self) -> Result<(), Failure> {
        let current = crate::sandbox::Sandbox::tool_objects(&self.executable)?;
        if current != self.objects
            || resources(&current)? != self.inputs.bundle_sha256
            || hash(&std::fs::read(&self.worker)?) != self.inputs.worker_sha256
        {
            return Err(Failure::unavailable(
                "Z3 or worker resource resolution/content changed after preparation",
            ));
        }
        Ok(())
    }
}
const SOLVER_TIMEOUT_MS: u64 = 10_000;
const SEED: u32 = 0;
fn limits() -> Limits {
    Limits {
        timeout: Duration::from_secs(15),
        capture_bytes: 65_536,
        output_bytes: 131_072,
    }
}
/// Exact host-selected finite method; resource bytes are frozen at preparation.
pub fn descriptor() -> io::Result<crate::plugin::Descriptor> {
    use crate::plugin::{Definition, Descriptor, Role};
    let implementation = serde_json::to_vec(&(
        include_str!("mod.rs"),
        include_str!("data.rs"),
        include_str!("terms.rs"),
    ))?;
    let limits = limits();
    Descriptor::new(Definition {
        id: "z3.finite.v1",
        role: Role::Verifier,
        version: "1",
        formats: vec!["finite-bitvectors-v1".into()],
        implementation: &implementation,
        configuration: serde_json::json!({"logic":"QF_BV","unsigned":true,"steps":1,"seed":SEED,"solver_timeout_ms":SOLVER_TIMEOUT_MS,"process_timeout_ms":limits.timeout.as_millis(),"capture_bytes":limits.capture_bytes,"output_bytes":limits.output_bytes,"maximum_input_bits":16,"independent_complete_enumeration":true}),
        input: Contract::schema()?,
        output: serde_json::to_value(schemars::schema_for!(Report))?,
    })
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
