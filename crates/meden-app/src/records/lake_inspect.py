"""Resolve and freshly elaborate the selected Lake source in the offline worker."""
import pathlib
import stat
import subprocess
import sys
import tempfile

module, *names = sys.argv[1:]
lake = '/analysis/lean/bin/lake'
selected = subprocess.run(
    [lake, 'query', '+' + module + ':lean'],
    check=True, stdout=subprocess.PIPE, text=True,
).stdout.strip()
source = pathlib.Path(selected)
canonical = source.resolve(strict=True)
relative = canonical.relative_to('/workspace')
if (not source.is_absolute() or source != canonical or source.suffix != '.lean'
        or '.git' in relative.parts or not stat.S_ISREG(source.lstat().st_mode)):
    raise RuntimeError('Lake did not resolve one regular nonsymlink workspace Lean source')
with tempfile.TemporaryDirectory(prefix='meden-inspect-') as directory:
    setup = pathlib.Path(directory) / 'setup.json'
    with setup.open('wb') as output:
        subprocess.run([lake, 'setup-file', selected], check=True, stdout=output)
    result = subprocess.run([
        lake, 'env', 'lean', '--run', '/analysis/library/inspect.lean',
        selected, str(setup), *names,
    ], check=False)
    sys.exit(result.returncode)
