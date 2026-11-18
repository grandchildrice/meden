"""Live public-TUI Finding -> reviewed repair -> independent offline check -> restart.

Synthetic inputs only. Model assessments, real Rust tests and native proof
coverage are recorded separately. This is not an accuracy or cold-install gate.
"""

from __future__ import annotations

import argparse
import codecs
import fcntl
import hashlib
import json
import os
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

import pyte

SOURCE = "pub fn eligible(balance: u64, stock: u64) -> bool { balance >= 150 && stock > 0 }\n"
REQUIREMENT = (
    "すべてのu64の残高と在庫について、残高が100以上かつ在庫が正のときだけ"
    "購入を許可し、それ以外は拒否する。"
)
CHECK = """mod implementation { include!("eligibility.rs"); }
#[test] fn independent_boundaries() {
    for (balance, stock, expected) in [
        (0,1,false),(99,1,false),(100,1,true),(149,1,true),(150,1,true),
        (100,0,false),(u64::MAX,0,false),(u64::MAX,u64::MAX,true)
    ] {
        assert_eq!(implementation::eligible(balance,stock),expected,
                   "balance={balance}, stock={stock}");
    }
}
"""


class Terminal:
    def __init__(self, command, env, timeout):
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
        self.initial = termios.tcgetattr(self.slave)
        self.screen = pyte.Screen(160, 40)
        self.stream = pyte.Stream(self.screen)
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.process = subprocess.Popen(
            command,
            stdin=self.slave,
            stdout=self.slave,
            stderr=self.slave,
            env=env,
            start_new_session=True,
        )
        self.timeout = timeout

    def view(self):
        return "\n".join(
            "".join(self.screen.buffer[y][x].data for x in range(self.screen.columns))
            for y in range(self.screen.lines)
        )

    def wait(self, predicate):
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            if predicate(self.view()):
                return
            if self.process.poll() is not None:
                raise RuntimeError(f"TUI exited: {self.process.returncode}\n{self.view()}")
            if select.select([self.master], [], [], 0.1)[0]:
                self.stream.feed(self.decoder.decode(os.read(self.master, 65536)))
        raise TimeoutError(self.view())

    def send(self, text):
        os.write(self.master, text.encode() if isinstance(text, str) else text)

    def command(self, text):
        self.send(b"\x1b[200~" + text.encode() + b"\x1b[201~\r")

    def quit(self):
        self.send(b"\x03")
        self.process.wait(timeout=5)
        return self.process.returncode == 0 and termios.tcgetattr(self.slave) == self.initial

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        os.close(self.master)
        os.close(self.slave)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    connection = parser.add_mutually_exclusive_group(required=True)
    connection.add_argument("--provider")
    connection.add_argument("--base-url", help="Explicit host-selected endpoint; CI uses loopback")
    parser.add_argument("--model", required=True)
    parser.add_argument(
        "--verifier-bundle",
        type=Path,
        help="Explicit installed verifier prerequisite; never a cold-install claim",
    )
    parser.add_argument("--judge-model")
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        parser.error("--out must be a new file")
    binary = args.binary.resolve()
    report = {
        "success": False,
        "provider": args.provider,
        "model": args.model,
        "judge_model": args.judge_model,
        "binary": str(binary),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "verifier_bundle": str(args.verifier_bundle)
        if args.verifier_bundle
        else os.environ.get("MEDEN_VERIFIER_BUNDLE"),
        "original_source": SOURCE,
        "requirement": REQUIREMENT,
        "screens": [],
        "scope": "Synthetic public-TUI repair; independent real offline Rust tests. "
        "No native proof receipt, general accuracy, local inference or cold install claim.",
    }
    started = time.monotonic()
    terminal = None
    with tempfile.TemporaryDirectory(prefix="meden-meaning-smoke-") as directory:
        root = Path(directory)
        project = root / "project"
        project.mkdir()
        source = project / "eligibility.rs"
        source.write_text(SOURCE)
        env = {**os.environ, "XDG_DATA_HOME": str(root / "data"), "TERM": "xterm-256color"}
        if args.verifier_bundle:
            env["MEDEN_VERIFIER_BUNDLE"] = str(args.verifier_bundle.resolve())
        command = [
            str(binary),
            "tui",
            "--workspace",
            str(project),
            "--model",
            args.model,
            "--provider" if args.provider else "--base-url",
            args.provider or args.base_url,
        ]

        if args.judge_model:
            command.extend(["--judge-model", args.judge_model])

        def milestone(name):
            report["screens"].append({"step": name, "screen": terminal.view()})
            print(name, flush=True)

        def history():
            paths = list((root / "data/meden/projects").glob("*/meaning-analysis.json"))
            return json.loads(paths[0].read_text())["entries"] if paths else []

        try:
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: (
                    "Connection details" in view
                    and (not args.judge_model or f"Judge model: {args.judge_model}" in view)
                    and "Allow for this session" in view
                )
            )
            milestone("processor review")
            terminal.send("y")
            terminal.wait(lambda view: "shift+enter" in view)
            terminal.command("/sources eligibility.rs")
            terminal.wait(lambda view: "Source graph ready" in view and "shift+enter" in view)
            terminal.command(f"/analyze eligibility.rs:1 eligible -- {REQUIREMENT}")
            terminal.wait(
                lambda view: (
                    "Candidate " in view
                    and "shift+enter" in view
                    or "Analysis stopped" in view
                    and "shift+enter" in view
                )
            )
            report["initial_analysis"] = history()[-1]
            if (
                report["initial_analysis"]["state"] != "complete"
                or report["initial_analysis"]["assessment"]["verdict"] != "mismatch"
            ):
                raise RuntimeError(
                    "Initial analysis did not establish the expected candidate mismatch"
                )
            milestone("initial candidate mismatch")
            terminal.command("/repair")
            terminal.wait(
                lambda view: (
                    "Apply once and reanalyze" in view
                    and "File: eligibility.rs" in view
                    or "Repair stopped" in view
                    and "shift+enter" in view
                )
            )
            if "Repair stopped" in terminal.view():
                raise RuntimeError("Repair proposal was not admitted")
            report["unchanged_before_approval"] = source.read_text() == SOURCE
            if not report["unchanged_before_approval"]:
                raise RuntimeError("Source changed before approval")
            milestone("reviewed frozen repair")
            terminal.send(b"\x1b[6~")
            terminal.wait(
                lambda view: "+pub fn eligible" in view and "Apply once and reanalyze" in view
            )
            milestone("reviewed source diff")
            terminal.send("y")
            terminal.wait(
                lambda view: (
                    "Repair applied and reanalyzed" in view
                    and "shift+enter" in view
                    or "Repair stopped" in view
                    and "shift+enter" in view
                )
            )
            milestone("repair outcome")
            report["history"] = history()
            report["reanalysis"] = report["history"][-1]
            report["final_source"] = source.read_text()
            if len(report["history"]) < 2 or report["reanalysis"]["state"] != "complete":
                raise RuntimeError("Repair did not complete with a new analysis")
            report["requirement_preserved"] = all(
                entry["input"]["requirement"] == REQUIREMENT for entry in report["history"]
            )
            check = subprocess.run(
                [str(binary), "exec", "--workspace", str(project)],
                input="cat > independent.rs <<'MEDEN_INDEPENDENT'\n"
                + CHECK
                + "MEDEN_INDEPENDENT\nrustc --test independent.rs -o independent-tests"
                + " && ./independent-tests\n",
                capture_output=True,
                text=True,
                env=env,
                timeout=60,
            )
            report["independent_host_exit_code"] = check.returncode
            report["independent_check"] = json.loads(check.stdout)
            report["independent_stderr"] = check.stderr
            report["first_terminal_restored"] = terminal.quit()
            terminal.close()
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: (
                    "Connection details" in view
                    and (not args.judge_model or f"Judge model: {args.judge_model}" in view)
                    and "Allow for this session" in view
                )
            )
            milestone("fresh review after restart")
            terminal.send("n")
            terminal.wait(lambda view: "Choose a model connection" in view)
            terminal.send(b"\x1b")
            terminal.wait(lambda view: "shift+enter" in view)
            terminal.command("/analysis")
            terminal.wait(lambda view: "Historical" in view and "shift+enter" in view)
            milestone("historical result without renewed approval")
            report["terminal_restored"] = terminal.quit()
            report["history_after_restart"] = history()
            report["success"] = bool(
                report["requirement_preserved"]
                and report["unchanged_before_approval"]
                and report["final_source"] != SOURCE
                and check.returncode == 0
                and report["independent_check"].get("result", {}).get("exit_code") == 0
                and report["first_terminal_restored"]
                and report["terminal_restored"]
            )
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
            report["history"] = history()
            report["final_source"] = source.read_text()
            if terminal:
                report["screens"].append({"step": "failure", "screen": terminal.view()})
        finally:
            if terminal:
                terminal.close()
            report["seconds"] = round(time.monotonic() - started, 3)
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
