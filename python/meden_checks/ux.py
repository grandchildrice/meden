"""Run required, deterministic user journeys and retain decoded terminal screens."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

import pytest

from meden_checks import find_meden

_STATE = pytest.StashKey[dict]()


def pytest_addoption(parser):
    group = parser.getgroup("meden experience")
    group.addoption("--ux-artifacts", type=Path, help="Save decoded PTY frames and UX verdicts")
    group.addoption("--ux-required", action="store_true",
                    help="Fail if UX cases skip or are absent")


def pytest_configure(config):
    root = config.getoption("--ux-artifacts")
    if root:
        root.mkdir(parents=True, exist_ok=True)
    config.stash[_STATE] = {"root": root, "results": {}, "selected": []}


@pytest.hookimpl(trylast=True)
def pytest_collection_modifyitems(config, items):
    config.stash[_STATE]["selected"] = [
        item.nodeid for item in items if item.get_closest_marker("ux")
    ]


@pytest.fixture(autouse=True)
def ux_screens(request, monkeypatch):
    """Observe terminal decoders, without changing input, timing or assertions.

    This records cells after PTY reads, not every rendered frame. Only synthetic
    fixtures should use this: decoded screens can contain conversation content.
    """
    root = request.config.stash[_STATE]["root"]
    if root is None:
        yield
        return
    import pyte

    node = request.node.nodeid
    slug = request.node.name[:70] + "-" + hashlib.sha256(node.encode()).hexdigest()[:10]
    destination = root / slug
    started = time.monotonic()
    original_feed = pyte.Stream.feed
    previous = {}
    count = size = 0
    truncated = False
    last = ""

    def feed(stream, data):
        nonlocal count, size, last, truncated
        result = original_feed(stream, data)
        screen = stream.listener
        # Read cells directly: pyte.display can fail on a clipped wide glyph.
        text = "\n".join(
            "".join(screen.buffer[y][x].data for x in range(screen.columns)).rstrip()
            for y in range(screen.lines)
        )
        if previous.get(id(stream)) != text:
            previous[id(stream)] = text
            last = text
            destination.mkdir(exist_ok=True)
            record = json.dumps({"seconds": round(time.monotonic() - started, 3),
                                 "columns": screen.columns, "rows": screen.lines,
                                 "screen": text}, ensure_ascii=False) + "\n"
            # Bound CI storage while retaining the last observed screen below.
            if size + len(record.encode()) <= 10_000_000:
                with (destination / "screens.jsonl").open("a") as output:
                    output.write(record)
                count += 1
                size += len(record.encode())
            else:
                truncated = True
        return result

    monkeypatch.setattr(pyte.Stream, "feed", feed)
    yield
    if last:
        (destination / "last-screen.txt").write_text(last + "\n")
        (destination / "trace.json").write_text(json.dumps({
            "test": node, "frames": count, "bytes": size,
            "truncated": truncated,
        }, indent=2) + "\n")


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item, call):
    outcome = yield
    report = outcome.get_result()
    if not item.get_closest_marker("ux"):
        return
    results = item.config.stash[_STATE]["results"]
    result = results.setdefault(item.nodeid, {"outcome": "passed", "seconds": 0})
    result["seconds"] += report.duration
    if report.failed or report.skipped or hasattr(report, "wasxfail"):
        result["outcome"] = "failed" if report.failed else "skipped"
        result["reason"] = str(report.longrepr)


def pytest_sessionfinish(session, exitstatus):
    state = session.config.stash[_STATE]
    passed = bool(state["selected"]) and all(
        state["results"].get(node, {}).get("outcome") == "passed"
        for node in state["selected"]
    )
    success = passed and exitstatus == 0
    if session.config.getoption("--ux-required") and not success and exitstatus == 0:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
    if state["root"]:
        binary = find_meden()
        identity = None
        if binary:
            path = Path(binary).resolve()
            identity = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        summary = {
            "binary": identity,
            "success": success, "inference": "scripted fixtures; no live-model validation",
            "selected": state["selected"], "results": state["results"],
        }
        (state["root"] / "summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
        )
        traces = {}
        for metadata in state["root"].glob("*/trace.json"):
            info = json.loads(metadata.read_text())
            traces[info["test"]] = [json.loads(line) for line in
                                   metadata.with_name("screens.jsonl").read_text().splitlines()]
        evidence = {"success": success, "tests": [
            {"name": node, **state["results"].get(node, {"outcome": "not run"}),
             "frames": traces.get(node, [])} for node in state["selected"]
        ]}
        template = Path(__file__).with_name("ux_view.html").read_text()
        # Script contents can include fixture source or model-controlled text.
        payload = json.dumps(evidence, ensure_ascii=False).replace("<", "\\u003c")
        (state["root"] / "index.html").write_text(
            template.replace("__MEDEN_EVIDENCE__", payload),
        )


def pytest_terminal_summary(terminalreporter, exitstatus, config):
    state = config.stash[_STATE]
    if config.getoption("--ux-required"):
        incomplete = [node for node in state["selected"]
                      if state["results"].get(node, {}).get("outcome") != "passed"]
        if incomplete or not state["selected"]:
            terminalreporter.write_sep("=", "UX gate requires every selected journey to pass")
            for node in incomplete:
                terminalreporter.write_line(node)
    if state["root"]:
        terminalreporter.write_line(f"UX screens and verdicts: {state['root']}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True, help="New artifact directory")
    args = parser.parse_args(argv)
    root = args.out.resolve()
    if root.exists():
        parser.error("--out must be a new directory; previous evidence is never overwritten")
    return subprocess.call([
        sys.executable, "-m", "pytest", "-q", "-m", "ux", "--ux-required",
        "--ux-artifacts", str(root), "--junitxml", str(root / "junit.xml"),
        "--basetemp", str(root / "work"),
    ], cwd=Path(__file__).resolve().parents[1])


if __name__ == "__main__":
    raise SystemExit(main())
