"""What each benchmark tool is allowed to do, and which of its arguments steer it.

Declaring this is the work a deployment does. Nothing is inferred: a tool with
no declaration is refused rather than waved through, because a tool whose
steering parameters nobody named is a tool whose steering parameters nothing
protects.

A parameter *steers* when it decides what happens in the world — a recipient,
an amount, a path, a password — rather than merely what is said. The subject
line of a transfer is carried; the IBAN it goes to is obeyed. That distinction
is the whole of the integrity rule, so it is the only judgement these tables
record.
"""

from __future__ import annotations

from typing import Any

#: One entry per tool: which parameters steer, whom the call discloses its
#: arguments to, what capabilities it needs, and who may read what it returns.
Declaration = dict[str, Any]

# These mirror the banking policy in CaMeL's own AgentDojo integration
# (`security_policies/banking.py`), so that a number measured here can be set
# beside a number measured there. CaMeL requires the *recipient* of a transfer
# to be trusted and leaves the amount, date and subject to the
# confidentiality check; marking those as steering too would refuse more and
# make the comparison flattering rather than fair.
BANKING: dict[str, Declaration] = {
    # Reading. Nothing here changes the world, so nothing steers.
    "get_iban": {"requires": ["secret.read[own-iban]"]},
    "get_balance": {},
    "get_most_recent_transactions": {},
    "get_scheduled_transactions": {},
    "get_user_info": {},
    "read_file": {"requires": ["fs.read[{file_path}]"]},
    # Acting. The recipient decides where the money goes; the amount, date and
    # subject travel with it and face the disclosure rule instead.
    "send_money": {
        "control": ["recipient"],
        "discloses_to": "recipient",
        "requires": ["net.egress[{recipient}]"],
    },
    "schedule_transaction": {
        "control": ["recipient"],
        "discloses_to": "recipient",
        "requires": ["net.egress[{recipient}]"],
    },
    "update_scheduled_transaction": {
        "control": ["recipient"],
        "discloses_to": "recipient",
        "requires": ["net.egress[{recipient}]"],
    },
    # Changing the account itself. Every field here is obeyed.
    "update_password": {"control": ["password"], "requires": ["secret.read[password]"]},
    "update_user_info": {
        "control": ["first_name", "last_name", "street", "city"],
        "requires": ["fs.write[user-info]"],
    },
}

SLACK: dict[str, Declaration] = {
    "get_channels": {},
    "read_channel_messages": {"control": ["channel"], "requires": ["fs.read[{channel}]"]},
    "read_inbox": {"control": ["user"], "requires": ["fs.read[inbox]"]},
    "get_users_in_channel": {"control": ["channel"]},
    "get_webpage": {"control": ["url"], "requires": ["net.egress[{url}]"]},
    "add_user_to_channel": {
        "control": ["user", "channel"],
        "requires": ["fs.write[{channel}]"],
    },
    "send_channel_message": {
        "control": ["channel"],
        "discloses_to": "channel",
        "requires": ["net.egress[{channel}]"],
    },
    "send_direct_message": {
        "control": ["recipient"],
        "discloses_to": "recipient",
        "requires": ["net.egress[{recipient}]"],
    },
    "invite_user_to_slack": {
        "control": ["user", "user_email"],
        "requires": ["net.egress[{user_email}]"],
    },
    "remove_user_from_slack": {"control": ["user"], "requires": ["fs.write[workspace]"]},
    "post_webpage": {
        "control": ["url"],
        "discloses_to": "url",
        "requires": ["net.egress[{url}]"],
    },
}

#: Every suite this knows how to declare.
SUITES: dict[str, dict[str, Declaration]] = {
    "banking": BANKING,
    "slack": SLACK,
}


class UndeclaredTool(Exception):
    """A tool the suite offers that nothing has declared.

    Raised rather than defaulted, because either default is wrong: treating
    every argument as steering refuses ordinary work, and treating none as
    steering is the hole the whole design exists to close.
    """


def tool_specs(runtime: Any, declarations: dict[str, Declaration]) -> list[dict[str, Any]]:
    """Describe a runtime's tools in the form the agent's protocol expects.

    Raises:
        UndeclaredTool: if the runtime offers a tool with no declaration.
    """
    specs = []
    for tool in runtime.functions.values():
        declaration = declarations.get(tool.name)
        if declaration is None:
            raise UndeclaredTool(
                f"{tool.name} has no declaration; add one to declarations.py saying "
                "which of its parameters steer the call"
            )
        steering = set(declaration.get("control", ()))
        unknown = steering - set(tool.parameters.model_fields)
        if unknown:
            raise UndeclaredTool(
                f"{tool.name} declares parameters it does not have: {sorted(unknown)}"
            )
        spec: dict[str, Any] = {
            "name": tool.name,
            "description": (tool.description or "").strip(),
            "params": [
                {"name": name, "control": name in steering} for name in tool.parameters.model_fields
            ],
            "requires": list(declaration.get("requires", ())),
        }
        if "discloses_to" in declaration:
            spec["discloses_to"] = declaration["discloses_to"]
        if "returns_readers" in declaration:
            spec["returns_readers"] = list(declaration["returns_readers"])
        specs.append(spec)
    return specs


#: A policy that grants every capability, so that what refuses a call is the
#: provenance of its arguments and nothing else.
#:
#: This is the configuration that measures the claim actually being made. A
#: realistic deployment would also restrict capabilities, and would refuse
#: more; crediting those refusals to the integrity rule would overstate it.
PERMISSIVE_POLICY: dict[str, Any] = {
    "name": "capabilities-granted",
    "grants": [
        "fs.read",
        "fs.write",
        "net.egress",
        "net.listen",
        "shell.exec",
        "env.read",
        "secret.read",
        "process.spawn",
        "package.install",
        "clipboard",
        "ui.control",
    ],
    "denies": [],
}
