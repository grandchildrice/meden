"""Public portable work: real Lean failure/recovery, clone, fresh approval and continuation."""

import json
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from test_lake_analysis import installed_lean
from test_tui_setup import Terminal, environment

from meden_checks import find_meden

pytestmark = pytest.mark.ux

VALID_PROOF = "theorem portable_identity (n : Nat) : n = n := by rfl\n"
REQUIREMENT = "Preserve the original statement for every natural number."
NEXT = "Prove the mixed event trace without weakening the requirement."
KEY = "PORTABLE-PROCESSOR-KEY-MUST-STAY-LOCAL"


def _work(binary, root, env, *arguments):
    result = subprocess.run(
        [binary, "work", "--workspace", str(root), *arguments],
        env=env,
        capture_output=True,
        text=True,
        timeout=90,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result


def _command(terminal, command, expected, *, picker=False, timeout=180):
    if picker:
        terminal.send("/")
        terminal.wait(lambda screen: "shift+enter" in screen)
        command = command.removeprefix("/")
    terminal.paste(command)
    terminal.wait(lambda screen: command[-25:] in screen)
    terminal.send("\r")
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        terminal.pump()
        screen = terminal.view()
        if (
            expected in screen
            and "shift+enter" in screen
            and command[-25:] not in "\n".join(screen.splitlines()[-5:])
        ):
            return screen
        assert terminal.process.poll() is None, screen
    pytest.fail(terminal.view())


def _git(root, *arguments):
    result = subprocess.run(
        ["git", *arguments], cwd=root, capture_output=True, text=True, timeout=60
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_portable_work_clones_history_checks_locally_and_requires_fresh_resume_approval(
    tmp_path, tui_verifier
):
    binary = str(find_meden())
    lean = installed_lean()
    root = tmp_path / "original"
    root.mkdir()
    (root / "lean-toolchain").write_text("leanprover/lean4:v4.35.0\n")
    (root / "lakefile.toml").write_text('name = "portable"\n[[lean_lib]]\nname = "Proof"\n')
    # Real compilation stays busy long enough to inspect the responsive TUI before failure.
    (root / "Proof.lean").write_text(
        "#eval IO.sleep 1200\ntheorem portable_identity (n : Nat) : n = n := by exact True.intro\n"
    )
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            if len(requests) == 1:
                message = {
                    "tool_calls": [
                        {
                            "id": "correct-proof",
                            "type": "function",
                            "function": {
                                "name": "write_file",
                                "arguments": json.dumps(
                                    {"path": "Proof.lean", "content": VALID_PROOF}
                                ),
                            },
                        }
                    ]
                }
            else:
                message = {"content": "PROOF-READY" if len(requests) == 2 else "PORTABLE-RESUMED"}
            payload = json.dumps(
                {"choices": [{"message": {"role": "assistant", **message}}]}
            ).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    env = environment(tmp_path)
    env["MEDEN_PROJECT_LEAN"] = str(lean)

    def arguments(workspace):
        return [
            binary,
            "tui",
            "--workspace",
            str(workspace),
            "--base-url",
            f"http://127.0.0.1:{server.server_port}/v1",
            "--model",
            "portable-fixture",
            "--api-key",
            KEY,
        ]

    terminal = Terminal(arguments(root), env, root)
    try:
        terminal.wait(lambda screen: "Connection details" in screen)
        terminal.send("n")
        terminal.wait(lambda screen: "Choose a model connection" in screen)
        _command(
            terminal,
            "/work init Portable proofs",
            "Portable workspace initialized",
            picker=True,
        )
        assert requests == []
        terminal.quit()
        terminal.close()
        _work(
            binary,
            root,
            env,
            "add",
            "--id",
            "gossip",
            "--title",
            "Mixed gossip trace",
            "--requirement",
            REQUIREMENT,
            "--module",
            "Proof",
            "--theorem",
            "portable_identity",
            "--next",
            NEXT,
        )

        terminal = Terminal(arguments(root), env, root)
        terminal.wait(lambda screen: "Connection details" in screen)
        terminal.send("n")
        terminal.wait(lambda screen: "Choose a model connection" in screen)
        terminal.send("/")
        terminal.wait(lambda screen: "shift+enter" in screen)
        terminal.paste("work check gossip")
        terminal.send("\r")
        terminal.wait(lambda screen: "Checking portable Lean work offline" in screen)
        assert requests == []
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            terminal.pump()
            if "Use /work ID" in terminal.view() and "shift+enter" in terminal.view():
                break
        else:
            pytest.fail(terminal.view())
        document_path = root / ".meden/workspace.json"
        failed = json.loads(document_path.read_text())
        checks = [event["check"] for event in failed["events"] if event["check"]]
        assert checks and checks[-1]["success"] is False
        assert "Proof.lean" in checks[-1]["stdout"] + checks[-1]["stderr"]
        _command(
            terminal,
            "/work note gossip -- Original requirement retained after compiler rejection.",
            "Work note saved for gossip",
        )

        # Recovery uses the public user turn and actual file tool, with fresh processor approval.
        terminal.paste("Correct Proof.lean to prove portable_identity for every Nat using rfl.")
        terminal.send("\r")
        terminal.wait(lambda screen: "Connection details" in screen)
        assert requests == []
        terminal.send("y")
        terminal.wait(lambda screen: "shift+enter" in screen)
        terminal.send("\r")
        terminal.wait(lambda screen: "PROOF-READY" in screen and "shift+enter" in screen)
        assert len(requests) == 2
        assert (root / "Proof.lean").read_text() == VALID_PROOF
        automatically_saved = json.loads(document_path.read_text())
        assert "PROOF-READY" in json.dumps(automatically_saved["conversations"])
        _command(terminal, "/work check gossip", "Offline Lean check completed for gossip")
        _command(terminal, "/work save", "Conversation checkpoint saved")
        document = json.loads(document_path.read_text())
        assert document["tasks"][0]["requirement"] == REQUIREMENT
        assert document["tasks"][0]["next"] == NEXT
        assert document["conversations"]
        assert "PROOF-READY" in json.dumps(document["conversations"])
        assert KEY not in document_path.read_text()
        assert len(requests) == 2
        terminal.quit()
        terminal.close()

        _git(root, "init")
        _git(
            root,
            "add",
            "Proof.lean",
            "lean-toolchain",
            "lakefile.toml",
            "lake-manifest.json",
            ".meden",
        )
        _git(
            root,
            "-c",
            "user.name=Portable test",
            "-c",
            "user.email=portable@example.invalid",
            "commit",
            "-m",
            "Retain proof work and portable history",
        )
        clone = tmp_path / "another-pc" / "project"
        clone.parent.mkdir()
        _git(tmp_path, "clone", "--no-local", str(root), str(clone))
        clone_env = environment(clone.parent)
        clone_env["MEDEN_PROJECT_LEAN"] = str(lean)
        assert clone_env["XDG_DATA_HOME"] != env["XDG_DATA_HOME"]
        terminal = Terminal(arguments(clone), clone_env, clone)
        terminal.wait(lambda screen: "Connection details" in screen)
        terminal.send("n")
        terminal.wait(lambda screen: "Choose a model connection" in screen)
        screen = _command(terminal, "/work gossip", "Saved conversations:", picker=True)
        assert "PROOF-READY" in screen
        assert "historical" in screen
        assert len(requests) == 2
        _command(terminal, "/status", "Specification has not been accepted")
        _command(terminal, "/work check gossip", "Offline Lean check completed for gossip")
        checked_clone = json.loads((clone / ".meden/workspace.json").read_text())
        completed = [event["check"] for event in checked_clone["events"] if event["check"]]
        assert len(completed) == 3 and completed[-1]["success"]
        assert "portable_identity" in completed[-1]["inspection"]
        assert len(requests) == 2, (
            "A local proof check must not require or call an inference processor"
        )

        terminal.paste("/work resume gossip")
        terminal.send("\r")
        terminal.wait(lambda screen: "Connection details" in screen)
        assert len(requests) == 2, (
            "Cloning and selecting a task must not restore processor authority"
        )
        terminal.paste("y")
        terminal.pump()
        assert len(requests) == 2
        terminal.send("y")
        terminal.wait(lambda screen: "shift+enter" in screen)
        terminal.send("\r")
        terminal.wait(lambda screen: "PORTABLE-RESUMED" in screen and "shift+enter" in screen)
        assert len(requests) == 3
        assert REQUIREMENT in json.dumps(requests[-1])
        assert NEXT in json.dumps(requests[-1])
        terminal.quit()
    finally:
        terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def test_malformed_portable_record_cannot_restore_authority_or_be_silently_replaced(
    tmp_path, tui_verifier, request
):
    root = tmp_path / "project"
    root.mkdir()
    (root / ".meden").mkdir()
    forged = '{"version":1,"name":"forged","accepted":true,"verified":true}'
    path = root / ".meden/workspace.json"
    path.write_text(forged)
    env = environment(tmp_path)
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            requests.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            payload = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": f"ORDINARY-CHAT-AVAILABLE-{len(requests)}",
                            }
                        }
                    ]
                }
            ).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    request.addfinalizer(lambda: thread.join(timeout=5))
    request.addfinalizer(server.server_close)
    request.addfinalizer(server.shutdown)
    args = [
        str(find_meden()),
        "tui",
        "--workspace",
        str(root),
        "--base-url",
        f"http://127.0.0.1:{server.server_port}/v1",
        "--model",
        "no-inference",
    ]
    for index in range(2):
        terminal = Terminal(args, env, root)
        try:
            terminal.wait(lambda screen: "Connection details" in screen)
            terminal.send("n")
            terminal.wait(lambda screen: "Choose a model connection" in screen)
            _command(terminal, "/work", "Portable work stopped", picker=True)
            _command(terminal, "/work init Replacement", "Portable work stopped")
            assert path.read_text() == forged
            _command(terminal, "/status", "Specification has not been accepted")
            assert len(requests) == index
            terminal.paste(
                "Keep ordinary conversation available; retain the malformed work record."
            )
            terminal.send("\r")
            terminal.wait(lambda screen: "Connection details" in screen)
            assert len(requests) == index
            terminal.send("y")
            terminal.wait(lambda screen: "shift+enter" in screen)
            terminal.send("\r")
            terminal.wait(
                lambda screen, turn=index + 1: (
                    f"ORDINARY-CHAT-AVAILABLE-{turn}" in screen and "shift+enter" in screen
                )
            )
            assert len(requests) == index + 1
            assert path.read_text() == forged
            terminal.quit()
        finally:
            terminal.close()


def test_portable_check_is_cancellable_without_a_processor_grant(tmp_path, tui_verifier):
    binary = str(find_meden())
    lean = installed_lean()
    root = tmp_path / "project"
    root.mkdir()
    (root / "lean-toolchain").write_text("leanprover/lean4:v4.35.0\n")
    (root / "lakefile.toml").write_text('name = "portable"\n[[lean_lib]]\nname = "Proof"\n')
    (root / "Proof.lean").write_text("#eval IO.sleep 5000\n" + VALID_PROOF)
    env = environment(tmp_path)
    env["MEDEN_PROJECT_LEAN"] = str(lean)
    _work(binary, root, env, "init", "--name", "Cancellable proofs")
    _work(
        binary,
        root,
        env,
        "add",
        "--id",
        "gossip",
        "--title",
        "Mixed trace",
        "--requirement",
        REQUIREMENT,
        "--module",
        "Proof",
        "--theorem",
        "portable_identity",
        "--next",
        NEXT,
    )
    terminal = Terminal(
        [
            binary,
            "tui",
            "--workspace",
            str(root),
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--model",
            "no-inference",
        ],
        env,
        root,
    )
    try:
        terminal.wait(lambda screen: "Connection details" in screen)
        terminal.send("n")
        terminal.wait(lambda screen: "Choose a model connection" in screen)
        terminal.send("/")
        terminal.wait(lambda screen: "shift+enter" in screen)
        terminal.paste("work check gossip")
        terminal.send("\r")
        terminal.wait(lambda screen: "Checking portable Lean work offline" in screen)
        path = root / ".meden/workspace.json"
        deadline = time.monotonic() + 10
        while not json.loads(path.read_text())["events"]:
            assert time.monotonic() < deadline
            terminal.pump()
        terminal.send("\x1b")
        terminal.wait(lambda screen: "Portable work stopped" in screen and "shift+enter" in screen)
        stopped = json.loads(path.read_text())
        assert not any(event["check"] and event["check"]["success"] for event in stopped["events"])
        # A new host command clears cancellation and explicitly retries the original theorem.
        _command(terminal, "/work check gossip", "Offline Lean check completed for gossip")
        checked = json.loads(path.read_text())
        assert checked["events"][-1]["check"]["success"]
        assert checked["tasks"][0]["requirement"] == REQUIREMENT
        _command(terminal, "/status", "Specification has not been accepted")
        terminal.quit()
    finally:
        terminal.close()
