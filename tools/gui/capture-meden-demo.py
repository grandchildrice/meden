#!/usr/bin/env python3
"""Capture only the requested demo's visible transcript, code and retained proof report."""
import argparse
import hashlib
import json
import os
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--project', type=Path, default=Path('../meden-demo'))
parser.add_argument('--state', type=Path)
args = parser.parse_args()
project = args.project.resolve()
key = hashlib.sha256(os.fsencode(project)).hexdigest()
data = Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share'))
state = args.state or data / 'meden/projects' / key
source_hash = hashlib.sha256((project / 'kernel.rs').read_bytes()).hexdigest()
artifacts = sorted((state / 'artifacts').glob('*/report.json'), key=lambda path: path.stat().st_mtime, reverse=True)
artifact = next(path.parent for path in artifacts if json.loads(path.read_text())['source_sha256'] == source_hash)
report = json.loads((artifact / 'report.json').read_text())
conversation_path = state / 'conversation.json'
history = json.loads(conversation_path.read_text())
# The presentation transcript excludes tool arguments, hidden system prompts and provider settings.
transcript = [{'role': item['speaker'], 'text': item['text']} for item in history['transcript'] if item['speaker'] in ('user', 'assistant')]
files = {name: (project / name).read_text() for name in ['kernel.rs', 'main.rs', 'README.md', 'Cargo.toml', 'tests/cli.rs']}
files.update({name: (artifact / name).read_text() for name in ['Spec.lean', 'Check.lean', 'Candidate.lean']})
snapshot = {
    'project': 'meden-demo', 'name': '自販機シミュレーター',
    'capturedAt': datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    'recordedAt': datetime.fromtimestamp(conversation_path.stat().st_mtime, timezone.utc).isoformat().replace("+00:00", "Z"),
    'sourceSha256': source_hash,
    'transcriptSha256': hashlib.sha256(json.dumps(transcript, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest(),
    'files': files, 'contract': json.loads((artifact / 'contract.json').read_text()),
    'report': {key: report[key] for key in ['source_sha256', 'bundle_sha256', 'checker_sha256', 'audit', 'scope']},
    'transcript': transcript,
}
output = Path('apps/web/src/fixtures/meden-demo/snapshot.json')
output.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + '\n')
print(f'{output}: {len(transcript)} visible messages, {len(snapshot["contract"]["properties"])} properties; kernel hash matches retained report')
