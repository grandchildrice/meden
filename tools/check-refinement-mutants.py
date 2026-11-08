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
        "model": "Transfer",
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
        "kernel": "export_control",
        "model": "Control",
        "proof": "ExportControlRefinement",
        "test": "export_control::tests::dispatch_requires_review_host_origin_and_a_fresh_attempt",
        "mutations": [
            (
                "unreviewed_approval",
                "(Phase::Reviewed, Origin::Host, Event::Approve)",
                "(Phase::Pending | Phase::Reviewed, Origin::Host, Event::Approve)",
            ),
            (
                "model_approval",
                "(Phase::Reviewed, Origin::Host, Event::Approve)",
                "(Phase::Reviewed, Origin::Model, Event::Approve)",
            ),
            (
                "review_reopens_terminal",
                "(_, Origin::Host, Event::Review) => phase",
                "(_, Origin::Host, Event::Review) => Phase::Reviewed",
            ),
            (
                "approval_does_not_close",
                "phase: Phase::Attempted,",
                "phase: Phase::Reviewed,",
            ),
        ],
    },
]


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
            ROOT / f"crates/meden-core/src/{group['kernel']}.rs",
            ROOT / f"lean/MedenSpec/Generated/{group['model']}.lean",
        ]:
            originals[path] = path.read_bytes()
    logs = ROOT / "target/verification/mutants"
    logs.mkdir(parents=True, exist_ok=True)
    require_success(["lake", "build", "MedenProof"], ROOT / "lean")
    try:
        for group in GROUPS:
            kernel = ROOT / f"crates/meden-core/src/{group['kernel']}.rs"
            for name, old, new in group["mutations"]:
                source = originals[kernel].decode()
                if source.count(old) != 1:
                    raise RuntimeError(f"{name}: expected exactly one mutation target")
                kernel.write_text(source.replace(old, new))
                test = run(["cargo", "test", "-p", "meden-core", group["test"], "--", "--exact"])
                test_output = test.stdout + test.stderr
                (logs / f"{name}-rust.txt").write_text(test_output)
                if test.returncode == 0 or "test result: FAILED" not in test_output:
                    raise RuntimeError(f"{name}: expected an observed Rust failure\n{test_output}")
                require_success(["bash", "tools/extract-transfer.sh", "--write"])
                require_success(
                    ["lake", "build", f"MedenSpec.Generated.{group['model']}"], ROOT / "lean"
                )
                result = run(
                    ["lake", "env", "lean", f"MedenSpec/{group['proof']}.lean"], ROOT / "lean"
                )
                output = result.stdout + result.stderr
                (logs / f"{name}.txt").write_text(output)
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
