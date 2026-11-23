//! Emit the owned native source projection schema for frontend generation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = match std::env::args().nth(1).as_deref() {
        None | Some("source") => schemars::schema_for!(meden_app::view::SourcePage),
        Some("directory-index") => schemars::schema_for!(meden_app::view::directory::Index),
        Some("directory-publication") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::directory::Directory>
        ),
        Some("inventory") => schemars::schema_for!(meden_app::view::inventory::Snapshot),
        Some("inventory-status") => schemars::schema_for!(meden_app::view::inventory::Status),
        Some("inventory-publication") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::inventory::Snapshot>
        ),
        Some("inventory-chunk") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::inventory::Chunk>
        ),
        Some("reader-index") => schemars::schema_for!(meden_app::view::readers::Index),
        Some("reader-publication") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::file::FileView>
        ),
        Some("file") => schemars::schema_for!(meden_app::view::file::FileView),
        Some("history") => schemars::schema_for!(meden_app::view::history::Snapshot<'static>),
        Some("history-reference") => schemars::schema_for!(meden_app::view::history::Reference),
        Some("source-index") => schemars::schema_for!(meden_app::view::sources::Index),
        Some("source-publication") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::SourcePage>
        ),
        Some(_) => return Err("Unknown read model".into()),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
