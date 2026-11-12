"""Live-model TUI check: fix a small program, run tests, inspect real effects.

The caller selects a live provider; CI also exercises this driver with a
scripted HTTP fixture to detect stale UI assumptions. It drives the released binary
through a PTY and saves the final source, test results and terminal output.
Run separately from score measurements to keep their worker count fixed.
"""

from __future__ import annotations

import argparse
import codecs
import hashlib
import json
import os
import select
import subprocess
import tempfile
import time
from pathlib import Path

import pyte


def main(argv: list[str] | None = None) -> int:
    import fcntl
    import pty
    import struct
    import termios

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--provider", required=True)
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
    independent_follow_up = """import unittest
from calc import subtract

class TestSubtract(unittest.TestCase):
    def test_subtract(self):
        self.assertEqual(subtract(7, 2), 5)
        self.assertEqual(subtract(-4, 3), -7)
        self.assertEqual(subtract(3, 3), 0)
"""
    screen = pyte.Screen(160, 40)
    terminal = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    frames = []

    def view():
        return "\n".join("".join(screen.buffer[y][x].data for x in range(screen.columns))
                         for y in range(screen.lines))
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="meden-tui-smoke-") as directory:
        source = Path(directory) / "source"
        source.mkdir()
        workspace = source
        data = Path(directory) / "data"
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
            str(source),
            "--provider",
            args.provider,
        ]
        if args.model:
            command += ["--model", args.model]
        env = {**os.environ, "TERM": "xterm-256color", "XDG_DATA_HOME": str(data)}
        process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env)

        def wait_for(predicate):
            while time.monotonic() - started < args.timeout:
                if predicate():
                    return
                if process.poll() is not None:
                    raise RuntimeError(f"TUI exited early: {process.returncode}")
                if select.select([master], [], [], 0.1)[0]:
                    chunk = os.read(master, 65536)
                    terminal.feed(decoder.decode(chunk))
            raise TimeoutError("TUI smoke deadline reached")

        try:
            # An explicit provider opens its native review directly. Inspect
            # current terminal cells, never old strings in the ANSI history.
            wait_for(lambda: "Connection details" in view())
            frames.append({"step": "processor review", "screen": view()})
            report["workspace"] = str(source)
            os.write(master, b"y")
            wait_for(lambda: "shift+enter" in view())
            prompt = (
                "Fix the implementation in calc.py so add returns the sum of its arguments. "
                "Do not modify test_calc.py. Run "
                "python3 -m unittest -v test_calc.py "
                "and verify that the tests pass."
            )
            os.write(master, prompt.encode() + b"\r")
            wait_for(lambda: "shift+enter" not in view())
            wait_for(lambda: "shift+enter" in view())
            frames.append({"step": "first turn complete", "screen": view()})
            report["agent_ran_passing_tests"] = (workspace / "passing-test.marker").exists()
            report["first_turn_source"] = (workspace / "calc.py").read_text()
            report["tests_unchanged_after_first_turn"] = test.read_text() == original_test
            (workspace / "passing-test.marker").unlink(missing_ok=True)
            # Reopen the selected repository rather than continuing only in
            # process memory. Saved preferences must still need fresh review.
            os.write(master, b"\x03")
            process.wait(timeout=5)
            report["first_exit_code"] = process.returncode
            report["terminal_restored_before_restart"] = termios.tcgetattr(slave) == initial
            screen.reset()
            process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env)
            wait_for(lambda: "Connection details" in view())
            frames.append({"step": "fresh review after restart", "screen": view()})
            os.write(master, b"y")
            wait_for(lambda: "shift+enter" in view())
            report["reopened_with_fresh_review"] = True
            follow_up = (
                "In the same workspace, add subtract(a, b) to calc.py. Preserve add and "
                "leave test_calc.py unchanged. Create test_subtract.py with unittest cases "
                "for subtract(7, 2) == 5, subtract(-4, 3) == -7, and subtract(3, 3) == 0. "
                "Run python3 -m unittest -v "
                "test_calc.py test_subtract.py and verify that both test files pass."
            )
            report["prompts"] = [prompt, follow_up]
            os.write(master, follow_up.encode() + b"\r")
            wait_for(lambda: "shift+enter" not in view())
            wait_for(lambda: "shift+enter" in view())
            frames.append({"step": "follow-up complete", "screen": view()})
            report["agent_retested_after_follow_up"] = (workspace / "passing-test.marker").exists()
            follow_up_test = workspace / "test_subtract.py"
            report["agent_wrote_follow_up_tests"] = follow_up_test.is_file()
            report["follow_up_tests"] = (
                follow_up_test.read_text() if follow_up_test.is_file() else None
            )
            report["tests_unchanged"] = test.read_text() == original_test
            report["final_source"] = (workspace / "calc.py").read_text()
            # Always use the original tests for the independent check.
            test.write_text(original_test)
            (workspace / "verify_subtract.py").write_text(independent_follow_up)
            check = subprocess.run(
                [str(binary), "exec", "--workspace", str(workspace)],
                input="python3 -m unittest -v test_calc.py verify_subtract.py",
                capture_output=True,
                text=True,
                timeout=15,
                env=env,
            )
            report["independent_test_exit_code"] = check.returncode
            check_record = json.loads(check.stdout)
            check_result = check_record.get("result", {})
            report["independent_test_output"] = (
                check_result.get("stdout", "") + check_result.get("stderr", "") + check.stderr
            )
            report["independent_workspace"] = check_record["workspace"]
            report["independent_worker_exit_code"] = check_result.get("exit_code")
            report["edited_selected_repository"] = (
                workspace == source and (source / "calc.py").read_text()
                != "def add(a, b):\n    return a - b\n"
            )
            os.write(master, b"\x03")
            process.wait(timeout=5)
            report["tui_exit_code"] = process.returncode
            report["terminal_restored"] = termios.tcgetattr(slave) == initial
            report["success"] = bool(
                report["agent_ran_passing_tests"]
                and report["tests_unchanged_after_first_turn"]
                and report["agent_retested_after_follow_up"]
                and report["agent_wrote_follow_up_tests"]
                and report["tests_unchanged"]
                and check.returncode == 0
                and report["independent_worker_exit_code"] == 0
                and report["edited_selected_repository"]
                and process.returncode == 0
                and report["terminal_restored"]
                and report["first_exit_code"] == 0
                and report["terminal_restored_before_restart"]
                and report["reopened_with_fresh_review"]
            )
        except (OSError, RuntimeError, TimeoutError, ValueError, KeyError,
                subprocess.SubprocessError) as error:
            report["error"] = str(error)
            frames.append({"step": "failure", "screen": view()})
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
    transcript.write_text("\n\n".join(
        frame["step"] + "\n" + frame["screen"] for frame in frames
    ) + "\n")
    report["transcript"] = str(transcript)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
