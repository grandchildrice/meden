use super::{Backend, Checked, Class, Control, Description, Failure, Inputs, PreparedCheck};
use meden_runtime::verification::finite as solver;
use serde_json::Value;
use std::{io, path::Path};
pub(super) struct Finite;
impl Backend for Finite {
    type Specification = solver::Contract;
    fn id(&self) -> &'static str {
        "z3.finite.v1"
    }
    fn schema(&self) -> io::Result<Value> {
        solver::Contract::schema()
    }
    fn decode(&self, text: &str) -> io::Result<Self::Specification> {
        solver::Contract::parse(text)
    }
    fn describe(&self, spec: &Self::Specification) -> Description {
        Description {
            name: spec.name.clone(),
            source: spec.source_path.clone(),
            interface: format!(
                "Simultaneous {}-bit transition: state ({}) × arguments ({}) → state ({})",
                spec.width,
                spec.fields.join(", "),
                spec.parameters.join(", "),
                spec.fields.join(", ")
            ),
            properties: spec
                .properties
                .iter()
                .map(|p| (p.name.clone(), p.description.clone(), p.statement.text()))
                .collect(),
            review: spec.review_text(),
            class: Class::Bounded,
            scope: spec.scope(),
        }
    }
    fn checker_id(&self, spec: &Self::Specification) -> io::Result<String> {
        spec.checker_id()
    }
    fn setup(&self) -> anyhow::Result<()> {
        let z3 = meden_runtime::sandbox::Sandbox::public_tool("z3")
            .ok_or_else(|| anyhow::anyhow!("Install public Z3 before selecting z3.finite.v1"))?;
        meden_runtime::sandbox::Sandbox::tool_objects(&z3)?;
        Ok(())
    }
    fn check_draft(
        &self,
        spec: &Self::Specification,
        _: &Path,
        control: &Control,
    ) -> Result<(), Failure> {
        spec.validate()
            .map_err(|e| Failure::new(meden_runtime::verification::FailureKind::Unverified, e))?;
        control.with_active(|| Ok(())).map_err(|e| {
            Failure::new(
                if e.kind() == io::ErrorKind::Interrupted {
                    meden_runtime::verification::FailureKind::Inconclusive
                } else {
                    meden_runtime::verification::FailureKind::Unavailable
                },
                e,
            )
        })
    }
    fn prepare(
        &self,
        spec: &Self::Specification,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure> {
        Ok(Box::new(solver::prepare(source, spec, worker, control)?))
    }
}
impl PreparedCheck for solver::Prepared {
    fn inputs(&self) -> &Inputs {
        self.inputs()
    }
    fn check(self: Box<Self>, control: &Control) -> Result<Checked, Failure> {
        (*self).check(control)
    }
}
