# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow", "pyte"]
# ///
"""Capture README screens from a real TUI and real offline counter verification.

uv run tools/capture-readme.py --font /path/to/JetBrainsMono-Regular.otf \
    --bundle /path/to/prepared/bundle.json

The model is a scripted, confined local fixture. It supplies the checked-in
counter contract/source; all acceptance, execution and evidence are real.
No remote inference or credential is used. Screens are never reconstructed
from invented tool results. Each PNG has its actual terminal text alongside it.
"""

import argparse
import codecs
import fcntl
import hashlib
import json
import os
import pty
import runpy
import select
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

import pyte

ROOT = Path(__file__).resolve().parents[1]
ENGINE = r"""
import json, socketserver, sys, time
from http.server import BaseHTTPRequestHandler
index = 0
def call(name, args):
    return {'id': str(index), 'type': 'function',
            'function': {'name': name, 'arguments': json.dumps(args)}}
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        self.send_response(200); self.send_header('Content-Length', '2'); self.end_headers()
        self.wfile.write(b'{}')
    def do_POST(self):
        global index
        self.rfile.read(int(self.headers['Content-Length']))
        if index == 0:
            reply = {'content': 'I will specify an overflow-safe counter for your review.',
                     'tool_calls': [call('propose_contract', {'contract': CONTRACT})]}
        elif index == 1:
            time.sleep(2)  # Keep the genuine pending-proof state visible for capture.
            reply = {'content': ('Implementing checked addition. '
                                 'Overflow leaves the counter unchanged.'),
                     'tool_calls': [call('write_file', {'path': 'counter.rs', 'content': SOURCE})]}
        elif index == 2:
            reply = {'content': 'Checking the implementation against the accepted specification.',
                     'tool_calls': [call('verify_project', {})]}
        elif index == 3:
            reply = {'content': 'The pure Rust transition is verified against your specification.\n'
                                'CLI and I/O are outside this proof; sending '
                                'still needs your approval.'}
        elif index == 4:
            reply = {'content': 'Proved for every u64 state and increment:\n'
                                '- The counter never decreases.\n'
                                '- Overflow preserves the original state; '
                                'the result stays in u64.\n'
                                '- Adding zero leaves the state unchanged.\n\n'
                                'Scope: counter.rs / increment. '
                                'CLI and I/O are outside this proof.'}
        else:
            raise AssertionError('unexpected model request')
        index += 1
        data = json.dumps({'choices': [{'message': {'role': 'assistant', **reply}}]}).encode()
        self.send_response(200)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host') + 1], Handler).serve_forever()
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/meden")
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=ROOT / "docs/assets")
    args = parser.parse_args()
    render = runpy.run_path(str(ROOT / "tools/capture-tui.py"))["render_screen"]
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    source = (ROOT / "verification/examples/counter.rs").read_text()
    with tempfile.TemporaryDirectory(prefix="meden-readme-") as temporary:
        root = Path(temporary)
        project = root / "counter"
        project.mkdir()
        engine = root / "engine"
        engine.write_text(f"#!/bin/python3\nCONTRACT={contract!r}\nSOURCE={source!r}\n" + ENGINE)
        engine.chmod(0o700)
        weights = root / "fixture.gguf"
        weights.write_bytes(b"scripted engine fixture; not model weights")
        env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor",
               "XDG_DATA_HOME": str(root / "data"), "HERMES_HOME": str(root / "absent"),
               "MEDEN_VERIFIER_BUNDLE": str(args.bundle.resolve())}
        env.pop("NO_COLOR", None)
        master, slave = pty.openpty()
        columns, rows = 108, 30
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        initial = termios.tcgetattr(slave)
        process = subprocess.Popen(
            [str(args.binary.resolve()), "tui", "--workspace", str(project),
             "--local-engine", str(engine), "--local-model", str(weights)],
            stdin=slave, stdout=slave, stderr=slave, env=env,
        )
        screen = pyte.Screen(columns, rows)
        stream = pyte.Stream(screen)
        decoder = codecs.getincrementaldecoder("utf-8")("replace")

        def text():
            return "\n".join(screen.display)

        def pump(predicate, timeout=30):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if select.select([master], [], [], .05)[0]:
                    stream.feed(decoder.decode(os.read(master, 65536)))
                if predicate():
                    return
                if process.poll() is not None or "Stopped:" in text():
                    raise RuntimeError(text())
            raise TimeoutError(text())

        def capture(name):
            # A read can stop halfway through a terminal redraw. Drain stable
            # frames before taking the cell snapshot, including its footer.
            deadline = time.monotonic() + .3
            while time.monotonic() < deadline:
                if select.select([master], [], [], .05)[0]:
                    stream.feed(decoder.decode(os.read(master, 65536)))
            render(screen, args.font, args.out / name)
            print(args.out / name, flush=True)

        try:
            pump(lambda: "What would you like to build?" in text() and "███" in text()
                 and "shift+enter" in text())
            capture("tui.png")
            os.write(master, b"Build a counter that never wraps or decreases.\r")
            pump(lambda: "y: Accept specification" in text())
            os.write(master, b"y")
            pump(lambda: "Implementation proof pending" in text() and "○ unproved" in text()
                 and "esc / ctrl-c cancel" in text() and "y: Accept specification" not in text())
            assert "VERIFIED project kernel" not in text()
            capture("evidence-unproven.png")
            pump(lambda: "VERIFIED project kernel" in text() and "shift+enter" in text(), 360)
            pump(lambda: "sending still needs your approval." in text())
            assert "✓ proved" in text() and "Checked package available" in text()
            storage = root / "data/meden/projects"
            artifacts = list(storage.glob("*/artifacts/kernel-*/report.json"))
            assert len(artifacts) == 1
            report = json.loads(artifacts[0].read_text())
            assert report["contract"] == contract
            assert report["source_sha256"] == hashlib.sha256(source.encode()).hexdigest()
            for name, digest in report["files"].items():
                actual = (artifacts[0].parent / name).read_bytes()
                assert hashlib.sha256(actual).hexdigest() == digest
            # Keep the final image focused on the guarantees, while retaining
            # the genuine verified controller state and the full chat history.
            os.write(master, b"\x0cWhat did the proof establish?\r")
            scope = "Scope: counter.rs / increment. CLI and I/O are outside this proof."
            pump(lambda: scope in text()
                 and "shift+enter" in text())
            assert "VERIFIED project kernel" in text() and "✓ proved" in text()
            capture("evidence-verified.png")
            os.write(master, b"/quit\r")
            deadline = time.monotonic() + 5
            while process.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], .05)[0]:
                    os.read(master, 65536)
            assert process.wait(timeout=5) == 0 and termios.tcgetattr(slave) == initial
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
