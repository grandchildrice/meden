"""Exercise split-weight preparation through the real isolated inference backend."""

import json
import os
import subprocess

import pytest

from meden_checks import find_meden

ENGINE = r'''
import json, os, pathlib, socketserver, sys
from http.server import BaseHTTPRequestHandler

model = pathlib.Path(sys.argv[sys.argv.index('--model') + 1])
assert model == pathlib.Path('/model-00001-of-00003.gguf'), str(model)
assert not pathlib.Path(HOST_DIRECTORY).exists()
assert not pathlib.Path('/model.gguf').exists()
assert not pathlib.Path('/host-only').exists()
assert os.readlink('/proc/self/ns/net') != HOST_NET
for index in range(1, 4):
    shard = pathlib.Path(f'/model-{index:05}-of-00003.gguf')
    assert shard.read_text() == f'shard-{index}'
    try: shard.write_text('changed')
    except OSError: pass
    else: raise AssertionError('weights are writable')
assert not pathlib.Path('/model-00004-of-00003.gguf').exists()
count = 0
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b'{}')
    def do_POST(self):
        global count
        json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        count += 1
        body = json.dumps({'choices':[{'message':{
            'role':'assistant', 'content':f'Private split weights ready, request {count}.'
        }, 'finish_reason':'stop'}]}).encode()
        self.send_response(200); self.send_header('Content-Length', str(len(body)))
        self.end_headers(); self.wfile.write(body)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1], Handler).serve_forever()
'''


def prepare(tmp_path, *, symlinks=False):
    source = tmp_path / 'source'
    source.mkdir()
    weights = tmp_path / 'weights'
    weights.mkdir()
    for index in range(1, 4):
        path = weights / f'private-model-名-{index:05}-of-00003.gguf'
        if symlinks:
            blob = tmp_path / f'blob-{index}'
            blob.write_text(f'shard-{index}')
            path.symlink_to(blob)
        else:
            path.write_text(f'shard-{index}')
    (weights / 'host-only').write_text('HOST-CREDENTIAL')
    (weights / 'private-model-名-00004-of-00003.gguf').write_text('unselected')
    engine = tmp_path / 'engine'
    engine.write_text(
        '#!/usr/bin/env python3\n'
        + f'HOST_DIRECTORY={str(weights)!r}\n'
        + f'HOST_NET={os.readlink("/proc/self/ns/net")!r}\n' + ENGINE
    )
    engine.chmod(0o700)
    return source, weights, engine


def run(tmp_path, source, engine, model):
    binary = find_meden()
    assert binary
    return subprocess.run(
        [binary, 'run', '--workspace', str(source), '--local-engine', str(engine),
         '--local-model', str(model)],
        input='{"instruction":"Describe readiness."}\n' * 2,
        env={**os.environ, 'XDG_DATA_HOME': str(tmp_path / 'data')},
        capture_output=True, text=True, timeout=30,
    )


@pytest.mark.parametrize('symlinks', [False, True], ids=['files', 'cache-symlinks'])
def test_split_weights_are_private_readonly_and_keep_a_warm_engine(tmp_path, symlinks):
    source, weights, engine = prepare(tmp_path, symlinks=symlinks)
    result = run(tmp_path, source, engine, weights / 'private-model-名-00001-of-00003.gguf')
    assert result.returncode == 0, result.stderr
    records = [json.loads(line) for line in result.stdout.splitlines()]
    done = [record for record in records if record['type'] == 'done']
    assert [record['outcome'] for record in done] == ['done', 'done'], records
    assert [record['answer'] for record in done] == [
        'Private split weights ready, request 1.', 'Private split weights ready, request 2.',
    ]
    for index in range(1, 4):
        shard = weights / f'private-model-名-{index:05}-of-00003.gguf'
        assert shard.read_text() == f'shard-{index}'
    assert (weights / 'host-only').read_text() == 'HOST-CREDENTIAL'


@pytest.mark.parametrize('failure', ['missing', 'directory', 'later-shard', 'zero', 'overflow'])
def test_invalid_split_weights_fail_before_engine_start(tmp_path, failure):
    source, weights, engine = prepare(tmp_path)
    model = weights / 'private-model-名-00001-of-00003.gguf'
    if failure in ['missing', 'directory']:
        other = weights / 'private-model-名-00002-of-00003.gguf'
        other.unlink()
        if failure == 'directory':
            other.mkdir()
    elif failure == 'later-shard':
        model = weights / 'private-model-名-00002-of-00003.gguf'
    else:
        model = weights / f'model-00001-of-{"00000" if failure == "zero" else "99999"}.gguf'
        model.write_text('invalid')
    result = run(tmp_path, source, engine, model)
    assert result.returncode != 0 and not result.stdout, result
    assert 'weights' in result.stderr, result.stderr
