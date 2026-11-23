//! Emit the owned native source projection schema for frontend generation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = match std::env::args().nth(1).as_deref() {
        None | Some("source") => schemars::schema_for!(meden_app::view::SourcePage),
        Some("file") => schemars::schema_for!(meden_app::view::file::FileView),
        Some("history") => schemars::schema_for!(meden_app::view::history::Snapshot<'static>),
        Some("history-reference") => schemars::schema_for!(meden_app::view::history::Reference),
        Some(_) => return Err("Unknown read model".into()),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
