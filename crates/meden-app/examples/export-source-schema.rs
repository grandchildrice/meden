//! Emit the owned native source projection schema for frontend generation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(meden_app::view::SourcePage))?
    );
    Ok(())
}
