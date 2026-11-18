//! GUI presentation for the shared candidate analysis service.
use super::{lock, View};
pub(super) use crate::analysis::{load, Record, State};
use anyhow::{Context, Result};

pub(super) fn invalidate(view: &mut View) {
    if let Some(record) = &mut view.analysis {
        if record.state == State::Complete {
            record.state = State::Stale;
        }
    }
}
pub(super) fn run(
    view: &std::sync::Mutex<View>,
    session: &crate::session::Session,
    input: crate::semantic::Input,
) -> Result<()> {
    let snapshot = lock(view)
        .source_snapshot
        .clone()
        .context("Read the source graph before meaning analysis")?;
    session.analyze(&snapshot, input, &|record| {
        let mut view = lock(view);
        view.activity = match record.state {
            State::Translating => "意味解析：宣言の自然言語化",
            State::Comparing => "意味解析：要件との比較",
            State::Complete => "意味解析が完了（モデルによる候補評価）",
            _ => "意味解析",
        }
        .into();
        view.analysis = Some(record.clone());
    })?;
    Ok(())
}
