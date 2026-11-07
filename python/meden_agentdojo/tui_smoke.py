"""Live-model TUI check: fix a small program, run tests, inspect real effects.

This is deliberately not a mocked model test. It drives the released binary
through a PTY and saves the final source, test results and terminal output.
Run separately from score measurements to keep their worker count fixed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import select
import shlex
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    import fcntl
    import pty
    import struct
    import termios

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--provider", default="hermes:ollama-cloud")
    parser.add_argument("--model", default=None)
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    binary = args.binary.resolve()
    report = {
        "binary": str(binary),
        "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "provider": args.provider,
        "model": args.model,
        "success": False,
    }
    original_test = """import unittest
from pathlib import Path
from calc import add

class TestAdd(unittest.TestCase):
    def test_add(self):
        self.assertEqual(add(2, 3), 5)
        self.assertEqual(add(-4, 3), -1)
        Path("passing-test.marker").write_text("passed")
"""
    output = bytearray()
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="meden-tui-smoke-") as directory:
        workspace = Path(directory)
        (workspace / "calc.py").write_text("def add(a, b):\n    return a - b\n")
        test = workspace / "test_calc.py"
        test.write_text(original_test)
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
        initial = termios.tcgetattr(slave)
        command = [
            str(binary),
            "tui",
            "--workspace",
            directory,
            "--provider",
            args.provider,
            "--allow-shell",
        ]
        if args.model:
            command += ["--model", args.model]
        env = {**os.environ, "TERM": "xterm-256color", "PYTHONDONTWRITEBYTECODE": "1"}
        env.pop("MEDEN_TUI_SELFTEST", None)
        process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env)

        def wait_for(predicate):
            while time.monotonic() - started < args.timeout:
                if predicate():
                    return
                if process.poll() is not None:
                    raise RuntimeError(f"TUI exited early: {process.returncode}")
                if select.select([master], [], [], 0.1)[0]:
                    chunk = os.read(master, 65536)
                    if len(output) < 2_000_000:
                        output.extend(chunk)
            raise TimeoutError("TUI smoke deadline reached")

        try:
            wait_for(lambda: b"choose" in output)
            os.write(master, b"\r")
            wait_for(lambda: b"alt-enter" in output)
            prompt = (
                "Fix the implementation in calc.py so add returns the sum of its arguments. "
                "Do not modify test_calc.py. Run "
                f"{shlex.quote(sys.executable)} -m unittest -v test_calc.py "
                "and verify that the tests pass."
            )
            os.write(master, prompt.encode() + b"\r")
            wait_for(lambda: output.count(b"alt-enter") >= 2)
            report["agent_ran_passing_tests"] = (workspace / "passing-test.marker").exists()
            report["tests_unchanged"] = test.read_text() == original_test
            report["final_source"] = (workspace / "calc.py").read_text()
            # Always use the original tests for the independent check.
            test.write_text(original_test)
            check = subprocess.run(
                [sys.executable, "-m", "unittest", "-v", "test_calc.py"],
                cwd=directory,
                capture_output=True,
                text=True,
                timeout=15,
                env=env,
            )
            report["independent_test_exit_code"] = check.returncode
            report["independent_test_output"] = check.stdout + check.stderr
            os.write(master, b"\x03")
            process.wait(timeout=5)
            report["tui_exit_code"] = process.returncode
            report["terminal_restored"] = termios.tcgetattr(slave) == initial
            report["success"] = bool(
                report["agent_ran_passing_tests"]
                and report["tests_unchanged"]
                and check.returncode == 0
                and process.returncode == 0
                and report["terminal_restored"]
            )
        except (OSError, RuntimeError, TimeoutError, subprocess.SubprocessError) as error:
            report["error"] = str(error)
        finally:
            if process.poll() is None:
                os.write(master, b"\x03\x03")
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            os.close(master)
            os.close(slave)
    report["seconds"] = time.monotonic() - started
    args.out.parent.mkdir(parents=True, exist_ok=True)
    transcript = args.out.with_suffix(".terminal.txt")
    transcript.write_text(re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", output.decode(errors="replace")))
    report["transcript"] = str(transcript)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
