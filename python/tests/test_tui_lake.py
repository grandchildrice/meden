"""Installed-Lake public TUI: discover, compare, refuse stale repair, recover, restart."""

import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from test_lake_analysis import installed_lean
from test_tui_setup import Terminal, environment

from meden_checks import find_meden
from meden_checks.lake_fixture import make_project


@pytest.mark.ux
def test_public_lake_journey_preserves_metadata_binding_and_history(tmp_path, tui_verifier):
    lean = installed_lean()
    root = tmp_path / "project"
    original = make_project(root)
    config = root / "lakefile.lean"
    config.write_text(config.read_text() + "  globs := #[.andSubmodules `Store]\n")
    # Later sibling configurations must stay absent from this package's frozen
    # filesystem, even when their complete bytes have already been prefetched.
    logic_config = root / "vendor/logic/lakefile.toml"
    logic_config.unlink()
    (root / "vendor/logic/lakefile.lean").write_text('''import Lake
open Lake DSL
run_cmd do
  if ← System.FilePath.pathExists "/workspace/.lake/packages/cached/sub/config/lakefile.toml" then
    throwError "A future sibling configuration became visible"
package logic where
  srcDir := "lean"
require base from "../base"
lean_lib Domain
''')
    manifest_path = root / "lake-manifest.json"
    manifest = json.loads(manifest_path.read_text())
    next(p for p in manifest["packages"] if p["name"] == "logic")["configFile"] = "lakefile.lean"
    manifest_path.write_text(json.dumps(manifest))
    requests = []
    hold_next = threading.Event()
    translating = threading.Event()
    release = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            name = body["tools"][0]["function"]["name"]
            if name == "record_informalization" and hold_next.is_set():
                hold_next.clear()
                translating.set()
                release.wait(20)
            if name == "record_informalization":
                data = dict(
                    lemmaName="Domain.eligible",
                    naturalLanguage="Balance at least 150 and positive stock permits purchase.",
                    preconditions="All Nat inputs",
                    postcondition="Exactly balance >= 150 and stock > 0",
                    scope="One decision",
                    strength="moderate",
                    confidence=0.95,
                )
            elif name == "record_comparison":
                data = {
                    "lemmaName": "Domain.eligible",
                    "match": False,
                    "discrepancy": "The requested threshold is 100, not 150.",
                    "weakeningType": "wrong-property",
                    "explanation": "Balances 100–149 differ.",
                }
            else:
                assert name == "propose_repair"
                data = dict(
                    file="vendor/base/source/Base.lean",
                    old_text="150",
                    new_text="100",
                    explanation="Change the shared purchase threshold to 100.",
                )
            payload = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "candidate",
                                        "type": "function",
                                        "function": {"name": name, "arguments": json.dumps(data)},
                                    }
                                ],
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
    env = environment(tmp_path)
    env.pop("MEDEN_ATLAS_LEAN", None)
    elan = tmp_path / "elan"
    (elan / "toolchains").mkdir(parents=True)
    version = lean.name.removeprefix("leanprover--lean4---")
    (elan / "toolchains" / f"leanprover--lean4---{version}").symlink_to(
        lean, target_is_directory=True
    )
    (root / "lean-toolchain").write_text(f"leanprover/lean4:{version}\n")
    env["ELAN_HOME"] = str(elan)
    args = [
        find_meden(),
        "tui",
        "--workspace",
        str(root),
        "--base-url",
        f"http://127.0.0.1:{server.server_port}/v1",
        "--model",
        "lake-fixture",
    ]
    terminal = Terminal(args, env, root)
    screens = {}

    def command(text, expected, count):
        failures = terminal.view().count("Source preparation stopped:")
        terminal.paste(text)
        terminal.send("\r")
        if text.startswith("/graph"):
            terminal.wait(lambda view: "Reading sources / preparing local compiler" in view)
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            terminal.pump()
            view = terminal.view()
            if (
                expected in view
                and "shift+enter" in view
                and len(requests) == count
                and ("Reading sources / preparing local compiler" not in view)
            ):
                screens[text] = view
                return
            if ("shift+enter" in view
                    and view.count("Source preparation stopped:") > failures):
                raise AssertionError(view)
        raise AssertionError(terminal.view())

    def entries():
        path = next((tmp_path / "data/meden/projects").glob("*/meaning-analysis.json"))
        return json.loads(path.read_text())["entries"]

    try:
        terminal.wait(
            lambda view: "Connection details" in view and "Allow for this session" in view
        )
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/graph", "Domain.eligible · vendor/logic/lean/Domain.lean:3", 0)
        assert "Source graph ready: 4 files" in terminal.view()
        assert "Lake package: shop" in terminal.view()
        assert "Extra.tag · .lake/packages/cached/sub/code/Extra.lean:2" in terminal.view()
        command(
            "/analyze Domain.eligible -- REQUIREMENT_ONLY: balance >= 100 and stock > 0",
            "Candidate Mismatch",
            2,
        )
        first = entries()[-1]
        assert "Base.threshold" in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY" not in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY" in json.dumps(requests[1])
        assert "elaborated" not in json.dumps(requests[1])
        terminal.paste("/repair")
        terminal.send("\r")
        terminal.wait(
            lambda view: (
                "vendor/base/source/Base.lean" in view and "Apply once and reanalyze" in view
            )
        )
        # Deliberate concurrent host fault, never an off-screen repair of the journey.
        manifest = root / "lake-manifest.json"
        manifest.write_text(json.dumps(json.loads(manifest.read_text()), indent=2))
        terminal.send("y")
        terminal.wait(
            lambda view: (
                "Workspace metadata changed" in view and "shift+enter" in view
            )
        )
        assert (root / "vendor/base/source/Base.lean").read_text() == original[
            "vendor/base/source/Base.lean"
        ]
        assert len(requests) == 3, "stale metadata must stop before edit or inference"
        command("/graph", "Source graph ready: 4 files", 3)
        command(
            "/analyze Domain.eligible -- REQUIREMENT_ONLY: balance >= 100 and stock > 0",
            "Candidate Mismatch",
            5,
        )
        latest = entries()[-1]
        assert latest["requirement_id"] == first["requirement_id"]
        assert latest["input"]["graph_revision"] != first["input"]["graph_revision"]
        assert latest["source_artifact"] != first["source_artifact"]
        hold_next.set()
        terminal.paste("/analyze Domain.eligible -- Test an inventory change")
        terminal.send("\r")
        terminal.wait(lambda view: "Translating" in view and "shift+enter" not in view)
        assert translating.wait(5)
        # A newly created glob directory changes the selected project module set.
        added = root / "src/lib/Store/Added.lean"
        added.parent.mkdir()
        added.write_text("namespace Store\ndef added : Nat := 7\nend Store\n")
        release.set()
        terminal.wait(
            lambda view: "Workspace source inventory changed" in view and "shift+enter" in view
        )
        command("/analysis", "Result: Stale", 7)
        assert entries()[-1]["assessment"] is None
        command("/graph missing.lean", "Analysis stopped", 7)
        assert "Source observation failed for missing.lean" in terminal.view()
        command("/analyze Domain.eligible -- no older graph", "Read the source graph first", 7)
        command("/graph", "Source graph ready: 5 files", 7)
        terminal.quit()
        terminal.close()
        terminal = Terminal(args, env, root)
        terminal.wait(lambda view: "Connection details" in view)
        terminal.send("n")
        terminal.wait(lambda view: "Choose a model connection" in view)
        terminal.send("\x1b")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/analysis", "Historical", 7)
        terminal.quit()
        assert not (root / ".lake/config").exists()
        (tmp_path / "lake-tui-observation.json").write_text(
            json.dumps(
                dict(
                    scope=(
                        "Actual installed Lake and public TUI; scripted inference, no proof receipt"
                    ),
                    screens=screens,
                    requests=requests,
                    entries=entries(),
                ),
                indent=2,
            )
        )
    finally:
        release.set()
        terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
