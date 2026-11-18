#!/usr/bin/env python3
"""Capture project messages/tool recordings, code and retained proof; exclude host prompts/config."""
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
# Preserve original tool events at their position AFTER the last visible message.
# Never copy the host-injected continuation prompt or connection configuration.
def content_text(value):
    if isinstance(value, str): return value
    if isinstance(value, list): return '\n'.join(item.get('text', '') for item in value if item.get('type') == 'text')
    return ''

def clean_output(text):
    try:
        value = json.loads(text)
    except (ValueError, TypeError):
        return text
    def redact(item):
        if isinstance(item, dict): return {key: redact(value) for key,value in item.items() if key not in ('session','workspace','connection','api_key','access_token')}
        if isinstance(item, list): return [redact(value) for value in item]
        return item
    return json.dumps(redact(value), ensure_ascii=False, indent=2)

calls = {}
active = None
cursor = 0
for index, message in enumerate(history['messages']):
    text = content_text(message.get('content'))
    if message['role'] in ('user','assistant') and text:
        found = next((i for i in range(cursor,len(transcript)) if transcript[i]['role'] == message['role'] and transcript[i]['text'] == text), None)
        if found is not None:
            active = transcript[found]
            cursor = found + 1
    if active is None: continue
    for call in message.get('tool_calls', []):
        function = call['function']
        if function['name'] not in ('list_files','read_file','write_file','edit_file','bash','project_status','propose_contract','verify_project'): continue
        event = {'id':call['id'],'name':function['name'],'arguments':function['arguments'],'output':'','status':'unknown','sourceIndex':index}
        calls[call['id']] = event
        active.setdefault('activities',[]).append(event)
    if message['role'] == 'tool' and message.get('tool_call_id') in calls:
        event = calls[message['tool_call_id']]
        event['output'] = clean_output(text)
        event['status'] = 'complete'
        try:
            value = json.loads(text)
            if isinstance(value,dict) and value.get('success') is False: event['status'] = 'failed'
            elif isinstance(value,dict) and value.get('verification_failure'): event['status'] = 'incomplete'
        except ValueError:
            if text.startswith(('contract specification not verified','error:','Error:')): event['status'] = 'failed'
recording = {'originalMessages':len(history['messages']),'toolCalls':len(calls),'reasoningStored':False}
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
    'transcript': transcript, 'recording': recording,
}
output = Path('apps/web/src/fixtures/meden-demo/snapshot.json')
output.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + '\n')
print(f'{output}: {len(transcript)} visible messages, {len(snapshot["contract"]["properties"])} properties; kernel hash matches retained report')
