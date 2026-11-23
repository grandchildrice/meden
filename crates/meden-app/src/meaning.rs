//! Deterministic meaning-rendering port. Each implementation owns its decoded
//! formal representation; no universal prover AST or executable payload is used.
use anyhow::Result;
use meden_runtime::plugin::{Descriptor, Role};
use meden_runtime::process::Control;

/// A completed presentation observation, distinct from a semantic match/proof.
#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct Reading {
    /// Bounded realized sentence for one exact input expression.
    #[schemars(length(min = 1, max = 60_000))]
    pub sentence: String,
    /// Selected renderer/resource identity for presentation provenance.
    #[schemars(length(min = 1, max = 256))]
    pub renderer: String,
}

/// Role contract with a validated backend-owned input type.
pub trait Renderer {
    /// Decoded formal expression accepted by this implementation.
    type Expression: schemars::JsonSchema;
    /// Immutable selected method and resource identity.
    fn descriptor(&self) -> Result<Descriptor>;
    /// Recheck the originally selected resources, without selecting replacements.
    fn validate(&self) -> Result<()>;
    /// Render one expression in its prepared isolated worker.
    fn render(&self, control: &Control, expression: Self::Expression) -> Result<Reading>;
}

/// Frozen typed renderer used by one complete materialization/cache request.
pub struct Bound<R: Renderer> {
    renderer: R,
    definition: Descriptor,
}
impl<R: Renderer> Bound<R> {
    /// Freeze the selected instance and check both typed schemas.
    pub fn new(renderer: R) -> Result<Self> {
        let definition = renderer.descriptor()?;
        anyhow::ensure!(
            definition.role() == Role::MeaningRenderer
                && definition
                    .accepts_schema(&serde_json::to_value(schemars::schema_for!(R::Expression))?)?
                && definition.accepts_output_schema(&serde_json::to_value(
                    schemars::schema_for!(Reading)
                )?)?,
            "Meaning renderer definition does not match its typed role"
        );
        let selected = Self {
            renderer,
            definition,
        };
        selected.validate()?;
        Ok(selected)
    }
    /// Original cache/provenance identity, never an evidence receipt.
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    /// Check resources at the start and end of a complete materialization batch.
    pub fn validate(&self) -> Result<()> {
        self.renderer.validate()?;
        anyhow::ensure!(
            self.renderer.descriptor()? == self.definition,
            "Meaning renderer definition changed after selection"
        );
        Ok(())
    }
    /// Check the original resources around a complete batch, without hashing
    /// every grammar once for every declaration. The caller cannot retain a
    /// batch beyond those checks.
    ///
    /// ```compile_fail
    /// use meden_app::meaning::{Batch, Bound, Renderer};
    /// use meden_runtime::process::Control;
    /// fn escape<'a, R: Renderer>(bound: &'a Bound<R>, control: &'a Control)
    ///     -> anyhow::Result<Batch<'a, R>> {
    ///     bound.batch(control, Ok)
    /// }
    /// ```
    pub fn batch<T>(
        &self,
        control: &Control,
        run: impl for<'a> FnOnce(Batch<'a, R>) -> Result<T>,
    ) -> Result<T> {
        control.with_active(|| Ok(()))?;
        self.validate()?;
        let output = run(Batch {
            bound: self,
            control,
        })?;
        self.validate()?;
        control.with_active(|| Ok(()))?;
        Ok(output)
    }
}

/// Caller scoped to the enclosing resource-validation batch.
pub struct Batch<'a, R: Renderer> {
    bound: &'a Bound<R>,
    control: &'a Control,
}
impl<R: Renderer> Batch<'_, R> {
    /// Dispatch one original typed expression without selecting another method.
    pub fn render(&self, expression: R::Expression) -> Result<Reading> {
        self.control.with_active(|| Ok(()))?;
        anyhow::ensure!(
            self.bound.renderer.descriptor()? == self.bound.definition,
            "Meaning renderer definition changed before dispatch"
        );
        let reading = self.bound.renderer.render(self.control, expression)?;
        anyhow::ensure!(
            self.bound.renderer.descriptor()? == self.bound.definition,
            "Meaning renderer definition changed during dispatch"
        );
        anyhow::ensure!(
            !reading.sentence.is_empty()
                && reading.sentence.len() <= 60_000
                && !reading.renderer.is_empty()
                && reading.renderer.len() <= 256,
            "Invalid meaning renderer reading"
        );
        self.control.with_active(|| Ok(()))?;
        Ok(reading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone)]
    struct Fake {
        phase: Arc<AtomicUsize>,
        resources: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
        change_during_call: bool,
    }
    impl Renderer for Fake {
        type Expression = String;
        fn descriptor(&self) -> Result<Descriptor> {
            Ok(Descriptor::new(meden_runtime::plugin::Definition {
                id: "fixture.renderer",
                role: Role::MeaningRenderer,
                version: "1",
                formats: vec!["fixture".into()],
                implementation: b"fixture renderer",
                configuration: serde_json::json!({"phase":self.phase.load(Ordering::SeqCst)}),
                input: serde_json::to_value(schemars::schema_for!(String))?,
                output: serde_json::to_value(schemars::schema_for!(Reading))?,
            })?)
        }
        fn validate(&self) -> Result<()> {
            anyhow::ensure!(
                self.resources.load(Ordering::SeqCst),
                "Original fixture resources changed"
            );
            Ok(())
        }
        fn render(&self, _: &Control, expression: String) -> Result<Reading> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.change_during_call {
                self.phase.store(1, Ordering::SeqCst);
            }
            Ok(Reading {
                sentence: expression,
                renderer: "fixture".into(),
            })
        }
    }
    fn fixture(change_during_call: bool) -> Fake {
        Fake {
            phase: Arc::new(AtomicUsize::new(0)),
            resources: Arc::new(AtomicBool::new(true)),
            calls: Arc::new(AtomicUsize::new(0)),
            change_during_call,
        }
    }
    #[test]
    fn batch_keeps_original_method_inputs_resources_and_cancellation() -> Result<()> {
        let implementation = fixture(false);
        let bound = Bound::new(implementation.clone())?;
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        let readings = bound.batch(&control, |batch| {
            Ok(vec![
                batch.render("first original expression".into())?,
                batch.render("second original expression".into())?,
            ])
        })?;
        assert_eq!(
            readings.first().map(|r| r.sentence.as_str()),
            Some("first original expression")
        );
        assert_eq!(implementation.calls.load(Ordering::SeqCst), 2);
        implementation.phase.store(1, Ordering::SeqCst);
        assert!(bound
            .batch(&control, |batch| batch.render("not dispatched".into()))
            .is_err());
        assert_eq!(implementation.calls.load(Ordering::SeqCst), 2);
        implementation.phase.store(0, Ordering::SeqCst);
        assert!(bound
            .batch(&control, |batch| {
                let reading = batch.render("not published".into())?;
                implementation.resources.store(false, Ordering::SeqCst);
                Ok(reading)
            })
            .is_err());
        implementation.resources.store(true, Ordering::SeqCst);
        control.cancel();
        assert!(bound
            .batch(&control, |batch| batch.render("cancelled".into()))
            .is_err());
        assert_eq!(implementation.calls.load(Ordering::SeqCst), 3);
        Ok(())
    }
    #[test]
    fn changed_method_during_rendering_and_invalid_readings_are_refused() -> Result<()> {
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        let bound = Bound::new(fixture(true))?;
        assert!(bound
            .batch(&control, |batch| batch
                .render("not attributed to the old method".into()))
            .is_err());
        let bound = Bound::new(fixture(false))?;
        assert!(bound
            .batch(&control, |batch| batch.render(String::new()))
            .is_err());
        assert!(bound
            .batch(&control, |batch| batch.render("x".repeat(60_001)))
            .is_err());
        Ok(())
    }
}
