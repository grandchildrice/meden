"""Compose a compiled graph with independently bounded declaration observations."""

import json
import subprocess
import sys
import tempfile
import time

roots = sys.argv[1:]
lean = "/analysis/lean/bin/lean"
library = "/analysis/library/"


def execute(module, arguments, seconds):
    # Stdout is a bounded JSON protocol; actual diagnostics stream to the host.
    with tempfile.TemporaryFile() as output:
        child = subprocess.Popen(
            [lean, "--run", library + module + ".lean", *arguments],
            stdout=output,
            stderr=sys.stderr,
        )
        deadline = time.monotonic() + seconds
        try:
            while child.poll() is None:
                if time.monotonic() >= deadline:
                    raise TimeoutError("Observation reached its wall-clock limit")
                if output.tell() > 8_388_608:
                    raise RuntimeError("Observation output exceeded 8 MiB")
                time.sleep(0.02)
            if child.returncode:
                raise RuntimeError(
                    "Observation compiler failed (exit " + str(child.returncode) + ")"
                )
            if output.tell() > 8_388_608:
                raise RuntimeError("Observation output exceeded 8 MiB")
            output.seek(0)
            return json.load(output)
        finally:
            if child.poll() is None:
                child.kill()
            child.wait()


graph = execute("MedenAtlas", roots, 600)
search_deadline = time.monotonic() + 120
for index, node in enumerate(graph["nodes"]):
    if node["semantic"]["checker"]["status"] == "not_proposition":
        node["semantic"]["checker"]["engine"] = "Plausible"
        continue
    print(
        "Counterexample observation "
        + str(index + 1)
        + "/"
        + str(len(graph["nodes"]))
        + ": "
        + node["id"],
        file=sys.stderr,
        flush=True,
    )
    try:
        remaining = search_deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("The file's 120-second observation budget was reached")
        checker = execute("MedenSearchMain", [node["id"], *roots], min(10, remaining))
    except (TimeoutError, RuntimeError, ValueError) as error:
        checker = {
            "engine": "Plausible",
            "status": "budget_exhausted"
            if isinstance(error, TimeoutError)
            else "unavailable",
            "diagnostic": str(error) + "; no counterexample verdict is available.",
            "candidates": [],
        }
        print(
            node["id"] + ": " + checker["diagnostic"] + " Continuing.",
            file=sys.stderr,
            flush=True,
        )
    if "mode" in node["semantic"]["checker"]:
        checker["mode"] = node["semantic"]["checker"]["mode"]
    node["semantic"]["checker"] = checker
print(
    "All declaration observations completed or recorded as inconclusive.",
    file=sys.stderr,
    flush=True,
)
print(json.dumps(graph, separators=(",", ":")))
