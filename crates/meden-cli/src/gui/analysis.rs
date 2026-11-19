//! GUI presentation for the shared candidate analysis service.
use super::{lock, View};
pub(super) use crate::analysis::{Record, State};
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
    invalidate(&mut lock(view));
    let result = session.analyze(&snapshot, input, &|record| {
        let mut view = lock(view);
        view.activity = match record.state {
            State::Translating => "意味解析：宣言の自然言語化",
            State::Comparing => "意味解析：要件との比較",
            State::Complete => "意味解析が完了（モデルによる候補評価）",
            _ => "意味解析",
        }
        .into();
        view.analysis = Some(record.clone());
    });
    lock(view).analysis_history = session.analysis_history()?;
    result?;
    Ok(())
}

pub(super) fn apply_repair(
    view: &std::sync::Mutex<View>,
    session: &crate::session::Session,
    queue: &mut crate::analysis::repair::Queue,
    reference: &crate::analysis::repair::Reference,
) -> Result<()> {
    {
        let mut view = lock(view);
        anyhow::ensure!(
            view.repair_review
                .as_ref()
                .is_some_and(|review| &review.reference == reference),
            "Repair review selection changed"
        );
        view.repair_review = None;
        view.source_snapshot = None;
        view.project.verified = false;
        invalidate(&mut view);
    }
    let result = session.apply_repair(queue, reference, &|record| {
        lock(view).analysis = Some(record.clone());
    });
    lock(view).analysis_history = session.analysis_history()?;
    let (snapshot, record) = result?;
    let mut view = lock(view);
    view.source_snapshot = Some(snapshot);
    view.analysis = Some(record);
    view.activity = "修正を適用し、元の要件で再解析しました（モデルによる候補評価）".into();
    Ok(())
}
