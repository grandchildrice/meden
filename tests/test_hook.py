from t_agent.capability import Capability, Requirement
from t_agent.policy import Policy
from t_agent.runtime.hook import CapabilityGate, ToolCall


def _send_email_sig(call: ToolCall) -> set[Requirement]:
    domain = str(call.arguments.get("to", "")).split("@")[-1]
    return {Requirement(Capability.NET_EGRESS, domain, evidence="send_email.to")}


def _read_file_sig(call: ToolCall) -> set[Requirement]:
    return {
        Requirement(
            Capability.FS_READ, str(call.arguments.get("path", "")), evidence="read_file.path"
        )
    }


def _gate() -> CapabilityGate:
    policy = Policy.from_strings(
        "workspace", ["fs.read[/workspace/*]", "net.egress[*.example.com]"]
    )
    g = CapabilityGate(policy)
    g.register("send_email", _send_email_sig)
    g.register("read_file", _read_file_sig)
    return g


def test_allowed_call_passes():
    r = _gate()(ToolCall("send_email", {"to": "bob@mail.example.com"}))
    assert not r.block


def test_exfiltration_to_attacker_domain_is_blocked():
    r = _gate()(ToolCall("send_email", {"to": "attacker@evil.invalid"}))
    assert r.block and "net.egress[evil.invalid]" in (r.reason or "")


def test_path_outside_workspace_is_blocked():
    r = _gate()(ToolCall("read_file", {"path": "/home/user/.ssh/id_rsa"}))
    assert r.block


def test_untyped_tool_is_blocked():
    r = _gate()(ToolCall("run_shell", {"cmd": "ls"}))
    assert r.block and "no capability signature" in (r.reason or "")


def test_audit_trail_records_every_decision():
    g = _gate()
    g(ToolCall("read_file", {"path": "/workspace/a.txt"}))
    g(ToolCall("run_shell", {}))
    assert [d.allowed for _, d in g.audit] == [True, False]
