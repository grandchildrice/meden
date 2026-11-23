use super::{
    Backend, Checked, Class, Control, Description, Failure, Inputs, PreparedCheck, DEFAULT_BACKEND,
};
use meden_runtime::verification::{kernel, Contract as Specification};
use serde_json::Value;
use std::{
    io,
    path::{Path, PathBuf},
};
pub(super) struct Refinement {
    pub bundle: Option<PathBuf>,
}
impl Refinement {
    fn resources(&self) -> Result<meden_runtime::verification::Bundle, Failure> {
        let path = self.bundle.as_ref().ok_or_else(|| {
            Failure::unavailable(
            "Local Rust-to-Lean tools are not prepared; run meden setup before an offline session")
        })?;
        meden_runtime::verification::Bundle::load(path).map_err(Failure::unavailable)
    }
}
impl Backend for Refinement {
    type Specification = Specification;
    fn id(&self) -> &'static str {
        DEFAULT_BACKEND
    }
    fn schema(&self) -> io::Result<Value> {
        Ok(Specification::schema())
    }
    fn decode(&self, text: &str) -> io::Result<Specification> {
        Specification::parse(text)
    }
    fn describe(&self, spec: &Specification) -> Description {
        Description{name:spec.name().into(),source:spec.source().into(),interface:spec.api(),properties:spec.properties().map(|(a,b,c)|(a.into(),b.into(),c.into())).collect(),review:spec.review_text(),class:Class::Refinement,scope:"Functional refinement of the accepted pure u64 transition and its listed properties; I/O, concurrency, persistence and whole-application correctness are excluded.".into()}
    }
    fn checker_id(&self, spec: &Specification) -> io::Result<String> {
        kernel::checker_id(spec)
    }
    fn setup(&self) -> anyhow::Result<()> {
        crate::verifier::prepare()?;
        Ok(())
    }
    fn check_draft(
        &self,
        spec: &Specification,
        worker: &Path,
        control: &Control,
    ) -> Result<(), Failure> {
        kernel::check_contract(spec, &self.resources()?, worker, control)
    }
    fn prepare(
        &self,
        spec: &Specification,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure> {
        Ok(Box::new(kernel::prepare_kernel(
            source,
            spec,
            &self.resources()?,
            worker,
            control,
        )?))
    }
}
impl PreparedCheck for kernel::PreparedKernel {
    fn inputs(&self) -> &Inputs {
        self.inputs()
    }
    fn check(self: Box<Self>, control: &Control) -> Result<Checked, Failure> {
        (*self).check(control)
    }
}
