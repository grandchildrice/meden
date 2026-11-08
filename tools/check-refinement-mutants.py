"""Require real Rust faults to fail unchanged refinement proofs.

The transition models must compile before a proof failure counts. All owned
source/model bytes are restored, including on interruption or failed checks.
"""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GROUPS = [
    {
        "kernel": "transfer",
        "model": "Control",
        "proof": "TransferRefinement",
        "test": "transfer::tests::changing_session_recipient_or_payload_cannot_spend_a_permit",
        "mutations": [
            ("request_mismatch", "permit.request == *request", "permit.request != *request"),
            ("expired_permit", "now < permit.expires", "now <= permit.expires"),
            ("spent_permit", "&& !permit.spent", "&& true"),
            ("missing_reservation", "permit.spent |= accepted", "permit.spent |= false"),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "EvidenceRefinement",
        "test": "evidence::tests::exact_current_evidence_is_required_for_each_obligation",
        "mutations": [
            (
                "untrusted_evidence",
                "origin != EvidenceOrigin::Checker",
                "origin != EvidenceOrigin::Checker && false",
            ),
            ("cancelled_evidence", "!receipt.active", "false"),
            ("stale_evidence", "receipt.subject != *current", "false"),
            (
                "kind_substitution",
                "*expected != receipt.obligation",
                "*expected != receipt.obligation && expected.kind == receipt.obligation.kind",
            ),
            ("reused_result", "receipt.position != self.completed", "false"),
            ("stale_progress", "self.completed = 0", "let _ = self.completed"),
            (
                "stale_delivery",
                "&& self.subject == *current",
                "&& (self.subject == *current || true)",
            ),
            ("unreviewed_contract", "self.phase != EvidencePhase::Reviewed", "false"),
        ],
    },
    {
        "kernel": "export_control",
        "model": "Control",
        "proof": "ExportControlRefinement",
        "test": "export_control::tests::dispatch_requires_review_host_origin_and_a_fresh_attempt",
        "mutations": [
            (
                "unreviewed_approval",
                "(Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
                "(Phase::Pending | Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
            ),
            (
                "model_approval",
                "(Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
                "(Phase::Reviewed, Origin::Model, ExportEvent::Approve)",
            ),
            (
                "review_reopens_terminal",
                "(_, Origin::Host, ExportEvent::Review) => phase",
                "(_, Origin::Host, ExportEvent::Review) => Phase::Reviewed",
            ),
            (
                "approval_does_not_close",
                "phase: Phase::Attempted,",
                "phase: Phase::Reviewed,",
            ),
        ],
    },
    {
        "kernel": "export_control",
        "model": "Control",
        "proof": "DispatchRefinement",
        "prerequisites": ["TransferRefinement", "ExportControlRefinement"],
        "test": "export_control::tests::effects_are_frozen_reviewed_session_bound_and_single_use",
        "mutations": [
            (
                "forged_effect_origin",
                "let decision = step(self.phase, origin, event);",
                "let decision = step(self.phase, { let _ = origin; Origin::Host }, event);",
            ),
            (
                "substituted_effect_payload",
                "Some(self.request.clone())",
                "Some(Request { request: Vec::new(), ..self.request.clone() })",
            ),
            (
                "uncommitted_effect_attempt",
                "self.phase = decision.phase;",
                "let _ = decision.phase;",
            ),
            ("missing_authorized_effect", "Some(self.request.clone())", "None"),
        ],
    },
    {
        "source": "crates/meden-cli/src/tui/approval.rs",
        "package": "meden-cli",
        "model": "Approval",
        "proof": "ApprovalRefinement",
        "test": "tui::approval::tests::only_unmodified_key_presses_can_approve",
        "mutations": [
            (
                "repeat_approval",
                "key.kind == KeyEventKind::Press",
                "(key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat)",
            ),
            ("modified_approval", "key.modifiers.is_empty()", "(key.modifiers.is_empty() || true)"),
            (
                "enter_approval",
                "key.code == KeyCode::Char('y')",
                "(key.code == KeyCode::Char('y') || key.code == KeyCode::Enter)",
            ),
            ("paste_approval", "} else {\n        false\n", "} else {\n        true\n"),
        ],
    },
    {
        "source": "crates/meden-cli/src/session.rs",
        "package": "meden-cli",
        "model": "Repair",
        "proof": "RepairRefinement",
        "test": "session::tests::budget_recovery_respects_host_authority",
        "mutations": [
            ("budget_skips_check", "Status::Done(_) | Status::Incomplete", "Status::Done(_)"),
            (
                "failure_restarts_check",
                "Status::Done(_) | Status::Incomplete",
                "Status::Done(_) | Status::Incomplete | Status::Failed(_)",
            ),
            ("unaccepted_check", "accepted && changed", "(accepted || true) && changed"),
            ("unchanged_check", "accepted && changed", "accepted && (changed || true)"),
            ("cancelled_check", "&& !cancelled", "&& (!cancelled || true)"),
        ],
    },
]


def source_path(group):
    if "source" in group:
        return ROOT / group["source"]
    return ROOT / f"crates/meden-core/src/{group['kernel']}.rs"


def run(args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)


def require_success(args, cwd=ROOT):
    result = run(args, cwd)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


def main():
    originals = {}
    for group in GROUPS:
        for path in [
            source_path(group),
            ROOT / f"lean/MedenSpec/Generated/{group['model']}.lean",
        ]:
            originals[path] = path.read_bytes()
    logs = ROOT / "target/verification/mutants"
    logs.mkdir(parents=True, exist_ok=True)
    require_success(["lake", "build", "MedenProof"], ROOT / "lean")
    try:
        for group in GROUPS:
            kernel = source_path(group)
            for name, old, new in group["mutations"]:
                source = originals[kernel].decode()
                if source.count(old) != 1:
                    raise RuntimeError(f"{name}: expected exactly one mutation target")
                kernel.write_text(source.replace(old, new))
                test = run(
                    [
                        "cargo",
                        "test",
                        "-p",
                        group.get("package", "meden-core"),
                        group["test"],
                        "--",
                        "--exact",
                    ]
                )
                test_output = test.stdout + test.stderr
                (logs / f"{name}-rust.txt").write_text(test_output)
                if test.returncode == 0 or "test result: FAILED" not in test_output:
                    raise RuntimeError(f"{name}: expected an observed Rust failure\n{test_output}")
                require_success(
                    ["bash", "tools/extract-transfer.sh", "--write", group["model"].lower()]
                )
                require_success(
                    ["lake", "build", f"MedenSpec.Generated.{group['model']}"], ROOT / "lean"
                )
                for prerequisite in group.get("prerequisites", []):
                    require_success(["lake", "build", f"MedenSpec.{prerequisite}"], ROOT / "lean")
                result = run(
                    ["lake", "env", "lean", f"MedenSpec/{group['proof']}.lean"], ROOT / "lean"
                )
                output = result.stdout + result.stderr
                (logs / f"{name}.txt").write_text(output)
                operational_errors = (
                    "maximum number of heartbeats",
                    "maximum recursion depth",
                    "stack overflow",
                    "Unknown constant",
                    "Unknown identifier",
                    "unknown module prefix",
                )
                if any(error.casefold() in output.casefold() for error in operational_errors):
                    raise RuntimeError(
                        f"{name}: operational proof failure does not count\n{output}"
                    )
                proof_errors = (
                    "unsolved goals",
                    "made no progress",
                    "error: Tactic ",
                    "error: Type mismatch",
                    "error: Application type mismatch",
                )
                if result.returncode == 0 or not any(error in output for error in proof_errors):
                    raise RuntimeError(f"{name}: expected a correctness proof failure\n{output}")
                print(f"{name}: refinement rejected the Rust fault", flush=True)
                for path, original in originals.items():
                    path.write_bytes(original)
    finally:
        for path, original in originals.items():
            path.write_bytes(original)
        require_success(["lake", "build", "MedenProof"], ROOT / "lean")


if __name__ == "__main__":
    main()
