"""Require faults in the actual Lean syntax checker to fail unchanged proofs.

Each isolated module must compile, fail executable guards and fail semantic
proof checking. The repository and its normal build products are never mutated.
"""

import json
import os
import subprocess
import tempfile
from hashlib import sha256
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEAN = ROOT / "lean"
SOURCE = ROOT / "verification/kernel/Terms.lean"
PROOF = LEAN / "MedenSpec/ContractSyntax.lean"
TEST = LEAN / "ContractSyntaxTest.lean"
MUTATIONS = [
    ("unknown_identifier", "unless allowed.contains n do", "unless allowed.contains n || true do"),
    (
        "unknown_kind",
        "unless allowedKinds.contains (kind.toString false) do",
        "unless allowedKinds.contains (kind.toString false) || true do",
    ),
    (
        "field_punning",
        '&& !hasAtom ":=" 128 stx then',
        '&& !hasAtom ":=" 128 stx && false then',
    ),
    (
        "record_defaults",
        '&& hasAtom ".." 128 stx then',
        '&& hasAtom ".." 128 stx && false then',
    ),
    (
        "unchecked_child",
        "for arg in args do checkSyntax allowed fields fuel arg",
        "for arg in args do if false then checkSyntax allowed fields fuel arg",
    ),
    ("unknown_label", "unless fields.contains n do", "unless fields.contains n || true do"),
    (
        "expanded_whitelist",
        '"null", "num", "termIfThenElse",',
        '"Lean.Parser.Term.byTactic", "null", "num", "termIfThenElse",',
    ),
    (
        "exhausted_budget",
        '| 0, _ => .error "mathematical term nesting exceeds the supported budget"',
        "| 0, _ => .ok ()",
    ),
]


def main():
    originals = {path: path.read_bytes() for path in (SOURCE, PROOF, TEST)}
    source = originals[SOURCE].decode()
    library_path = subprocess.check_output(
        ["lake", "env", "printenv", "LEAN_PATH"], cwd=LEAN, text=True
    ).strip()
    logs = ROOT / "target/verification/contract-syntax-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    evidence = []
    with tempfile.TemporaryDirectory(prefix="meden-syntax-") as scratch:
        temp = Path(scratch)
        env = {**os.environ, "LEAN_PATH": f"{temp}:{library_path}"}

        def run(name, command, accepted):
            result = subprocess.run(
                command, cwd=LEAN, env=env, text=True, stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, timeout=180, check=False,
            )
            (logs / f"{name}.txt").write_text(result.stdout)
            if (result.returncode == 0) != accepted:
                raise RuntimeError(f"unexpected exit for {name}: see {logs / f'{name}.txt'}")
            return result.stdout

        for name, before, after in [("baseline", None, None), *MUTATIONS, ("restored", None, None)]:
            mutated = source
            if before is not None:
                if source.count(before) != 1:
                    raise RuntimeError(f"{name}: mutation is not unique")
                mutated = source.replace(before, after)
            (temp / "Terms.lean").write_text(mutated)
            run(name + "-module", ["lean", "-R", str(temp), "-o", str(temp / "Terms.olean"),
                                    str(temp / "Terms.lean")], True)
            normal = before is None
            test_log = run(name + "-guards", ["lean", str(TEST)], normal)
            proof_log = run(name + "-proof", ["lean", str(PROOF)], normal)
            if not normal:
                if "did not evaluate to `true`" not in test_log:
                    raise RuntimeError(f"{name}: executable guard failure missing")
                if not any(marker in proof_log for marker in (
                    "unsolved goals", "Tactic `decide` failed", "Tactic `simp` failed",
                    "Tactic `decide` proved that the proposition",
                    "Tactic `exact` failed", "Type mismatch", "Application type mismatch",
                    "Tactic `split` failed", "Tactic `subst` failed",
                    "Tactic `assumption` failed",
                )):
                    raise RuntimeError(f"{name}: semantic proof failure missing")
                if any(marker in proof_log + test_log for marker in (
                    "unknownIdentifier", "Unknown constant", "unknown module", "out of memory",
                    "Unknown identifier", "Unknown tactic", "unknown tactic", "Unknown module prefix",
                    "maximum recursion depth", "maximum number of heartbeats", "tactic execution",
                    "maximum number of steps exceeded",
                    "no goals to be solved", "No goals to be solved", "unexpected token",
                )):
                    raise RuntimeError(f"{name}: invalid or resource-limited rejection")
                evidence.append({
                    "fault": name, "source_sha256": sha256(mutated.encode()).hexdigest(),
                })
                print(f"{name}: compiled module, rejected guards and unchanged proof", flush=True)
    if any(path.read_bytes() != data for path, data in originals.items()):
        raise RuntimeError("repository inputs changed during fault checks")
    (logs / "summary.json").write_text(json.dumps({
        "source_sha256": sha256(originals[SOURCE]).hexdigest(),
        "proof_sha256": sha256(originals[PROOF]).hexdigest(),
        "guards_sha256": sha256(originals[TEST]).hexdigest(),
        "faults": evidence, "baseline_and_restored_passed": True,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
