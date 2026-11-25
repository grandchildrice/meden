//! Compiler-only method attribution; peer repair/composition changes do not invalidate compilation.
pub(crate) fn compiler(
    id: &str,
    implementation: &[u8],
    atlas: &crate::atlas::Configuration,
    configuration: &serde_json::Value,
) -> anyhow::Result<meden_runtime::plugin::Descriptor> {
    let implementation = [
        implementation,
        include_bytes!("snapshot.rs"),
        include_bytes!("../../analysis/collect.rs"),
        include_bytes!("../../analysis/layout.rs"),
        include_bytes!("../../analysis/observe.rs"),
        include_bytes!("../../analysis/sources.rs"),
        include_bytes!("../../identity/snapshots.rs"),
        include_bytes!("../../compiler.rs"),
        include_bytes!("../../explanation.rs"),
        include_bytes!("../../explanation/cache.rs"),
        include_bytes!("../../adapters/lean/cache.rs"),
        include_bytes!("../../adapters/lean/pipeline.rs"),
        include_bytes!("definition.rs"),
        include_bytes!("../../atlas/mod.rs"),
        include_bytes!("../../atlas/resources.rs"),
        include_bytes!("../../../../meden-runtime/src/sandbox.rs"),
        include_bytes!("../../atlas/project.rs"),
        include_bytes!("../../atlas/lake.rs"),
        include_bytes!("../../atlas/graph.rs"),
        include_bytes!("../../atlas/frozen.rs"),
    ]
    .concat();
    Ok(meden_runtime::plugin::Descriptor::new(
        meden_runtime::plugin::Definition {
            id,
            role: meden_runtime::plugin::Role::Compiler,
            version: "1",
            formats: vec!["lean-module-v1".into()],
            implementation: &implementation,
            configuration: serde_json::json!({"method":configuration,"resources_sha256":atlas.current_resources()?}),
            input: crate::compiler::input_schema()?,
            output: crate::compiler::output_schema::<crate::adapters::lean::Snapshot>()?,
        },
    )?)
}
