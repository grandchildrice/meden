"""Observe exact selected declarations, independently of graph compilation."""
import json
import subprocess
import sys
import tempfile
import time

request = json.loads(sys.stdin.buffer.read(8_388_609))
settings = request["settings"]
deadline = time.monotonic() + settings["file_seconds"]
checks = {}
targets = request["targets"]
for index, target in enumerate(targets):
    print(f"Counterexample observation {index + 1}/{len(targets)}: {target}", file=sys.stderr, flush=True)
    try:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("The file's observation budget was reached")
        with tempfile.TemporaryFile() as output:
            child = subprocess.Popen(
                ["/analysis/lean/bin/lean", "--run", "/analysis/library/MedenSearchMain.lean",
                 json.dumps({"seed": settings["seed"]}), target, *request["modules"]],
                stdout=output, stderr=sys.stderr,
            )
            expires = time.monotonic() + min(settings["declaration_seconds"], remaining)
            try:
                while child.poll() is None:
                    if time.monotonic() >= expires:
                        raise TimeoutError("Observation reached its wall-clock limit")
                    if output.tell() > 131_072:
                        raise RuntimeError("Observation output exceeded 128 KiB")
                    time.sleep(0.02)
                if child.returncode:
                    raise RuntimeError(f"Observation compiler failed (exit {child.returncode})")
                if output.tell() > 131_072:
                    raise RuntimeError("Observation output exceeded 128 KiB")
                output.seek(0)
                checks[target] = json.load(output)
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait()
    except (TimeoutError, RuntimeError, ValueError) as error:
        checks[target] = {"engine": "Plausible", "status": "budget_exhausted" if isinstance(error, TimeoutError) else "unavailable",
                          "diagnostic": str(error) + "; no counterexample verdict is available.", "candidates": []}
        print(f"{target}: {checks[target]['diagnostic']} Continuing.", file=sys.stderr, flush=True)
print("All declaration observations completed or recorded as inconclusive.", file=sys.stderr, flush=True)
print(json.dumps({"checks": checks}, separators=(",", ":")))
