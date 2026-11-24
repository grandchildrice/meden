//! Shared host command vocabulary. Decoding a command cannot approve a model,
//! schedule an effect or manufacture evidence; native adapters interpret it
//! through the existing application services and consuming kernels.
use serde::{Deserialize, Serialize};

/// Presentation category for an actual host-selected operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Conversation,
    Inspection,
    Analysis,
    Preview,
    Workspace,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    InitializeWork {
        name: String,
    },
    PutWork {
        work: crate::records::Work,
    },
    NoteWork {
        id: String,
        text: String,
        next: Option<String>,
    },
    CheckWork {
        id: String,
    },
    ResumeWork {
        id: String,
    },
    SaveWork,
    ApproveProcessor {
        connection_revision: crate::view::sequence::Sequence,
        endpoint: String,
        model: String,
        judge: Option<crate::model::Binding>,
    },
    SelectConnection {
        provider: String,
        model: String,
    },
    ListModels {
        provider: String,
    },
    RefreshWorkspace,
    RefreshConnections,
    RevokeProcessor,
    Instruction {
        text: String,
    },
    Cancel,
    RefreshStatus,
    PrepareAtlas,
    AtlasSnapshot {
        paths: Vec<String>,
    },
    ReviewExport {
        reference: crate::view::delivery::Reference,
        offset: u32,
    },
    SendExport {
        reference: crate::view::delivery::Reference,
        serial: crate::view::sequence::Sequence,
    },
    RejectExport {
        reference: crate::view::delivery::Reference,
        serial: crate::view::sequence::Sequence,
    },
    CloseExport {
        reference: crate::view::delivery::Reference,
        serial: crate::view::sequence::Sequence,
    },
    CapturePreview {
        root: String,
        entry: String,
    },
    SourceSnapshot {
        paths: Vec<String>,
    },
    PrepareRepair {
        finding_id: String,
    },
    ApplyRepair {
        reference: crate::analysis::repair::Reference,
    },
    RejectRepair {
        reference: crate::analysis::repair::Reference,
    },
    SelectAnalysis {
        id: String,
    },
    AnalyzeMeaning {
        input: crate::semantic::Input,
    },
    ReviewContract {
        reference: crate::view::evidence::Reference,
    },
    AcceptContract {
        reference: crate::view::evidence::Reference,
    },
    RejectContract {
        reference: crate::view::evidence::Reference,
    },
    Inspect {
        path: String,
        directory: bool,
    },
}

impl Command {
    /// Classify the original command without executing it.
    pub fn operation(&self) -> Operation {
        match self {
            Self::Instruction { .. } | Self::ResumeWork { .. } | Self::AcceptContract { .. } => {
                Operation::Conversation
            }
            Self::Inspect { .. } => Operation::Inspection,
            Self::CapturePreview { .. } => Operation::Preview,
            Self::AtlasSnapshot { .. }
            | Self::SourceSnapshot { .. }
            | Self::PrepareAtlas
            | Self::AnalyzeMeaning { .. }
            | Self::SelectAnalysis { .. }
            | Self::PrepareRepair { .. }
            | Self::ApplyRepair { .. }
            | Self::RejectRepair { .. } => Operation::Analysis,
            Self::InitializeWork { .. }
            | Self::PutWork { .. }
            | Self::NoteWork { .. }
            | Self::CheckWork { .. }
            | Self::SaveWork
            | Self::ApproveProcessor { .. }
            | Self::SelectConnection { .. }
            | Self::ListModels { .. }
            | Self::RefreshWorkspace
            | Self::RefreshConnections
            | Self::RevokeProcessor
            | Self::Cancel
            | Self::RefreshStatus
            | Self::ReviewExport { .. }
            | Self::SendExport { .. }
            | Self::RejectExport { .. }
            | Self::CloseExport { .. }
            | Self::ReviewContract { .. }
            | Self::RejectContract { .. } => Operation::Workspace,
        }
    }

    /// Whether the original command requires the host's current model binding.
    /// This classification grants nothing; the adapter checks actual gateways.
    pub fn uses_model(&self) -> bool {
        match self {
            Self::Instruction { .. }
            | Self::AnalyzeMeaning { .. }
            | Self::PrepareRepair { .. }
            | Self::ResumeWork { .. } => true,
            Self::InitializeWork { .. }
            | Self::PutWork { .. }
            | Self::NoteWork { .. }
            | Self::CheckWork { .. }
            | Self::SaveWork
            | Self::ApproveProcessor { .. }
            | Self::SelectConnection { .. }
            | Self::ListModels { .. }
            | Self::RefreshWorkspace
            | Self::RefreshConnections
            | Self::RevokeProcessor
            | Self::Cancel
            | Self::RefreshStatus
            | Self::PrepareAtlas
            | Self::AtlasSnapshot { .. }
            | Self::ReviewExport { .. }
            | Self::SendExport { .. }
            | Self::RejectExport { .. }
            | Self::CloseExport { .. }
            | Self::CapturePreview { .. }
            | Self::SourceSnapshot { .. }
            | Self::ApplyRepair { .. }
            | Self::RejectRepair { .. }
            | Self::SelectAnalysis { .. }
            | Self::ReviewContract { .. }
            | Self::AcceptContract { .. }
            | Self::RejectContract { .. }
            | Self::Inspect { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn original_command_paths_survive_round_trip(path in any::<String>(), directory in any::<bool>()) {
            let original = serde_json::json!({"type":"inspect","path":path,"directory":directory});
            let command: Command = serde_json::from_value(original.clone()).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(command.operation(), Operation::Inspection);
            prop_assert!(!command.uses_model());
            prop_assert_eq!(serde_json::to_value(command).map_err(|e| TestCaseError::fail(e.to_string()))?, original);
        }
    }
    #[test]
    fn approval_commands_decode_only_exact_positive_original_counters() -> Result<()> {
        let original = serde_json::json!({"type":"approve_processor","connection_revision":u64::MAX.to_string(),"endpoint":"https://original.invalid/v1","model":"original","judge":{"endpoint":"https://judge.invalid/v1","model":"judge"}});
        let command: Command = serde_json::from_value(original.clone())?;
        match &command {
            Command::ApproveProcessor {
                connection_revision,
                ..
            } => assert_eq!(connection_revision.value(), u64::MAX),
            _ => anyhow::bail!("original command tag changed"),
        }
        assert_eq!(serde_json::to_value(command)?, original);
        for value in [
            serde_json::json!(1),
            serde_json::json!(null),
            serde_json::json!("0"),
            serde_json::json!("01"),
            serde_json::json!("18446744073709551616"),
        ] {
            let mut invalid = original.clone();
            *invalid
                .get_mut("connection_revision")
                .ok_or_else(|| anyhow::anyhow!("missing counter"))? = value;
            assert!(serde_json::from_value::<Command>(invalid).is_err());
        }
        let mut extra = original;
        extra
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("missing command"))?
            .insert("approved".into(), true.into());
        assert!(serde_json::from_value::<Command>(extra).is_err());
        assert!(serde_json::from_value::<Command>(
            serde_json::json!({"type":"run_unknown_backend"})
        )
        .is_err());
        Ok(())
    }
}
