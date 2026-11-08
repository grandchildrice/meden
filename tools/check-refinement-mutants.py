"""Require real Rust faults to fail the unchanged refinement proof.

Run after the normal extraction/build checks. Local use: uv run this file.
Only the owned working files below change; their exact bytes are restored.
"""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KERNEL = ROOT / "crates/meden-core/src/transfer.rs"
MODEL = ROOT / "lean/MedenSpec/Generated/Transfer.lean"
MUTATIONS = [
    ("request_mismatch", "permit.request == *request", "permit.request != *request"),
    ("expired_permit", "now < permit.expires", "now <= permit.expires"),
    ("spent_permit", "&& !permit.spent", "&& true"),
    ("missing_reservation", "permit.spent |= accepted", "permit.spent |= false"),
]


def run(args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)


def require_success(args, cwd=ROOT):
    result = run(args, cwd)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


def main():
    original_kernel, original_model = KERNEL.read_bytes(), MODEL.read_bytes()
    logs = ROOT / "target/verification/mutants"
    logs.mkdir(parents=True, exist_ok=True)
    try:
        for name, old, new in MUTATIONS:
            source = original_kernel.decode()
            if source.count(old) != 1:
                raise RuntimeError(f"{name}: expected exactly one mutation target")
            KERNEL.write_text(source.replace(old, new))
            test = run(
                [
                    "cargo",
                    "test",
                    "-p",
                    "meden-core",
                    "transfer::tests::changing_session_recipient_or_payload_cannot_spend_a_permit",
                    "--",
                    "--exact",
                ]
            )
            test_output = test.stdout + test.stderr
            (logs / f"{name}-rust.txt").write_text(test_output)
            if test.returncode == 0 or "test result: FAILED" not in test_output:
                raise RuntimeError(f"{name}: expected an observed Rust failure\n{test_output}")
            require_success(["bash", "tools/extract-transfer.sh", "--write"])
            # A broken extractor, missing import or unsupported model is not
            # evidence that the correctness theorem rejected a semantic fault.
            require_success(["lake", "build", "MedenSpec.Generated.Transfer"], ROOT / "lean")
            result = run(
                ["lake", "env", "lean", "MedenSpec/TransferRefinement.lean"], ROOT / "lean"
            )
            output = result.stdout + result.stderr
            (logs / f"{name}.txt").write_text(output)
            proof_errors = ("unsolved goals", "made no progress", "error: Tactic ")
            if result.returncode == 0 or not any(error in output for error in proof_errors):
                raise RuntimeError(f"{name}: expected a correctness proof failure\n{output}")
            print(f"{name}: refinement rejected the Rust fault", flush=True)
    finally:
        KERNEL.write_bytes(original_kernel)
        MODEL.write_bytes(original_model)
        require_success(["lake", "build", "MedenProof"], ROOT / "lean")


if __name__ == "__main__":
    main()
