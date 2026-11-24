"""Fingerprint Lean source/configuration inputs, inside the offline worker."""
import hashlib
import os
import stat

digest = hashlib.sha256()
count = size = 0
for directory, children, files in os.walk('/workspace', followlinks=False):
    children[:] = sorted(name for name in children if name not in {
        '.git'
    })
    for child in children:
        if os.path.islink(os.path.join(directory, child)):
            raise RuntimeError('Lean source directory is a symlink; prepare ordinary local files')
    for name in sorted(files):
        if not (
            name.endswith('.lean')
            or name in {'lean-toolchain', 'lakefile.toml', 'lake-manifest.json'}
        ):
            continue
        path = os.path.join(directory, name)
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, 'rb') as source:
            info = os.fstat(source.fileno())
            if not stat.S_ISREG(info.st_mode):
                raise RuntimeError('Lean input is not a regular file')
            data = source.read(16 * 1024 * 1024 + 1)
        count += 1
        size += len(data)
        if count > 25000 or len(data) > 16 * 1024 * 1024 or size > 256 * 1024 * 1024:
            raise RuntimeError('Lean input fingerprint exceeds bounds')
        relative = os.fsencode(os.path.relpath(path, '/workspace'))
        digest.update(len(relative).to_bytes(8, 'big'))
        digest.update(relative)
        digest.update(len(data).to_bytes(8, 'big'))
        digest.update(data)
if count == 0:
    raise RuntimeError('No Lean source or project configuration found')
print('sha256:' + digest.hexdigest())
