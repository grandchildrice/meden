import pytest

from tagent_agentdojo import CapabilityGate, Requirement, TagentClient, ToolCall, find_tagent

pytestmark = pytest.mark.skipif(find_tagent() is None, reason="tagent binary not built")


def _send_email(call: ToolCall) -> list[Requirement]:
    domain = str(call.arguments.get("to", "")).split("@")[-1]
    return [Requirement("net.egress", domain, "send_email.to")]


def _read_file(call: ToolCall) -> list[Requirement]:
    return [Requirement("fs.read", str(call.arguments.get("path", "")), "read_file.path")]


def _gate() -> CapabilityGate:
    g = CapabilityGate(
        policy={"name": "ws", "grants": ["fs.read[/workspace/*]", "net.egress[*.example.com]"]},
        client=TagentClient(),
    )
    g.register("send_email", _send_email)
    g.register("read_file", _read_file)
    return g


def test_allowed_call_passes():
    assert not _gate()(ToolCall("send_email", {"to": "bob@mail.example.com"})).block


def test_exfiltration_is_blocked_with_reason():
    r = _gate()(ToolCall("send_email", {"to": "attacker@evil.invalid"}))
    assert r.block and "net.egress[evil.invalid]" in (r.reason or "")


def test_path_outside_workspace_is_blocked():
    assert _gate()(ToolCall("read_file", {"path": "/home/u/.ssh/id_rsa"})).block


def test_untyped_tool_is_blocked():
    r = _gate()(ToolCall("run_shell", {"cmd": "ls"}))
    assert r.block and "no capability signature" in (r.reason or "")


def test_audit_trail():
    g = _gate()
    g(ToolCall("read_file", {"path": "/workspace/a.txt"}))
    g(ToolCall("run_shell", {}))
    assert [d.allowed for _, d in g.audit] == [True, False]
