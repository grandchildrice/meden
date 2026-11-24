//! Emit the owned native source projection schema for frontend generation.
fn output<T: schemars::JsonSchema>() -> schemars::Schema {
    // A nullable serialized field is still present unless its serializer skips it.
    schemars::generate::SchemaSettings::default()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = match std::env::args().nth(1).as_deref() {
        Some("session-state") => output::<meden_app::view::session::State<'static>>(),
        Some("command") => schemars::schema_for!(meden_app::command::Command),
        Some("sequence") => schemars::schema_for!(meden_app::view::sequence::Sequence),
        Some("delivery-reference") => output::<meden_app::view::delivery::Reference>(),
        Some("delivery-summary") => output::<meden_app::view::delivery::Summary>(),
        Some("delivery-review") => output::<meden_app::view::delivery::Review>(),
        Some("delivery-outcome") => output::<meden_app::view::delivery::Outcome>(),
        Some("preview-status") => output::<meden_app::view::preview::Status>(),
        Some("preview-publication") => output::<meden_app::view::preview::Snapshot>(),
        Some("verification-job-status") => output::<meden_app::view::verification_jobs::Status>(),
        Some("verification-job-publication") => {
            output::<meden_app::view::verification_jobs::Snapshot>()
        }
        None | Some("source") => schemars::schema_for!(meden_app::view::SourcePage),
        Some("evidence-status") => output::<meden_app::view::evidence::Status>(),
        Some("evidence-publication") => output::<meden_app::view::evidence::Snapshot>(),
        Some("evidence-reference") => schemars::schema_for!(meden_app::view::evidence::Reference),
        Some("repair-review") => output::<meden_app::view::repairs::Data<'static>>(),
        Some("repair-reference") => schemars::schema_for!(meden_app::analysis::repair::Reference),
        Some("analyses-status") => output::<meden_app::view::analyses::Status>(),
        Some("analysis-current") => {
            output::<meden_app::view::analyses::CurrentPublication<'static>>()
        }
        Some("analysis-history") => {
            output::<meden_app::view::analyses::HistoryPublication<'static>>()
        }
        Some("meaning-record") => output::<meden_app::view::analyses::Record<'static>>(),
        Some("meaning-input") => schemars::schema_for!(meden_app::semantic::Input),
        Some("preparation-index") => schemars::schema_for!(meden_app::view::jobs::Index),
        Some("preparation-log") => schemars::schema_for!(
            meden_app::view::publication::Envelope<meden_app::view::jobs::Log>
        ),
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
