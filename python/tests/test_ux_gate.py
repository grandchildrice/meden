"""A skipped or absent UX journey must not produce green evidence."""

import json
import subprocess
import sys

import pytest


@pytest.mark.parametrize("case,expected", [("pass", 0), ("fail", 1), ("skip", 1), ("empty", 5)])
def test_ux_gate_requires_executed_passing_cases(tmp_path, case, expected):
    source = "import pytest\npytestmark = pytest.mark.ux\n"
    source += {
        "pass": ("def test_journey():\n"
                 "    import pyte\n"
                 "    screen = pyte.Screen(40, 3)\n"
                 "    pyte.Stream(screen).feed('hello </script>')\n"
                 "    assert 'hello </script>' in screen.display[0]\n"),
        "fail": "def test_journey(): assert False\n",
        "skip": "def test_journey(): pytest.skip('missing prerequisite')\n",
        "empty": "",
    }[case]
    (tmp_path / "test_fixture.py").write_text(source)
    out = tmp_path / "evidence"
    run = subprocess.run([
        sys.executable, "-m", "pytest", "-q", "-p", "meden_checks.ux",
        "-o", "markers=ux: fixture journey", "--ux-required", "--ux-artifacts", str(out),
    ], cwd=tmp_path, text=True, capture_output=True, timeout=20)
    assert run.returncode == expected, run.stdout + run.stderr
    summary = json.loads((out / "summary.json").read_text())
    assert summary["success"] is (case == "pass")
    page = (out / "index.html").read_text()
    assert "__MEDEN_EVIDENCE__" not in page and 'id="journey"' in page
    if case == "pass":
        payload = page.split('<script id="evidence" type="application/json">')[1].split(
            "</script>", 1,
        )[0]
        evidence = json.loads(payload)
        assert "hello </script>" in evidence["tests"][0]["frames"][0]["screen"]
        assert "hello </script>" not in payload, "terminal text escaped its JSON script"
        assert "hello </script>" in next(out.glob("*/last-screen.txt")).read_text()
