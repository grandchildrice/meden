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
# Where these are stricter than CaMeL, and why it is said out loud
# ----------------------------------------------------------------
# CaMeL's policies are disjunctive: an untrusted recipient is accepted if
# everything being sent is readable by that recipient. This runtime's
# integrity rule has no such escape — a parameter that steers must be
# trusted, full stop — so on three kinds of tool it refuses where CaMeL
# allows:
#
#   send_email / send_direct_message   CaMeL: trusted recipient OR the body
#                                      is readable by whoever it names.
#   get_webpage                        CaMeL: the url need only be public.
#   reschedule_calendar_event,
#   append_to_file                     CaMeL consults the environment for who
#                                      may read the event or the file; this
#                                      adapter cannot, so it requires the
#                                      handle itself to be trusted.
#
# Every one of those makes this side refuse more, not less. A utility number
# measured here is therefore a floor against CaMeL's, and an attack-success
# number is not flattered by the difference.

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
    # Reading. CaMeL treats all of these as having no side effect, so neither
    # does this: a tool that only looks does not steer anything.
    "get_channels": {},
    "read_channel_messages": {"requires": ["fs.read[{channel}]"]},
    "read_inbox": {"requires": ["fs.read[inbox]"]},
    "get_users_in_channel": {},
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


#: One tool per line, mirroring CaMeL's `travel.py`. Everything that only
#: looks something up steers nothing; a reservation is steered by what is
#: being reserved and when.
TRAVEL: dict[str, Declaration] = {
    # Looking things up.
    "get_user_information": {},
    "get_all_hotels_in_city": {},
    "get_hotels_prices": {},
    "get_rating_reviews_for_hotels": {},
    "get_hotels_address": {},
    "get_all_restaurants_in_city": {},
    "get_cuisine_type_for_restaurants": {},
    "get_restaurants_address": {},
    "get_rating_reviews_for_restaurants": {},
    "get_dietary_restrictions_for_all_restaurants": {},
    "get_contact_information_for_restaurants": {},
    "get_price_for_restaurants": {},
    "check_restaurant_opening_hours": {},
    "get_all_car_rental_companies_in_city": {},
    "get_car_types_available": {},
    "get_rating_reviews_for_car_rental": {},
    "get_car_fuel_options": {},
    "get_car_rental_address": {},
    "get_car_price_per_day": {},
    "get_flight_information": {},
    "search_calendar_events": {},
    "get_day_calendar_events": {},
    # Booking. CaMeL requires every field of a reservation to be trusted,
    # because a reservation made on an injected page's terms is still a
    # reservation.
    "reserve_hotel": {
        "control": ["hotel", "start_day", "end_day"],
        "requires": ["net.egress[booking]"],
    },
    "reserve_car_rental": {
        "control": ["company", "start_time", "end_time"],
        "requires": ["net.egress[booking]"],
    },
    "reserve_restaurant": {
        "control": ["restaurant", "start_time"],
        "requires": ["net.egress[booking]"],
    },
    # The calendar and the outbox.
    "create_calendar_event": {
        "control": ["participants"],
        "discloses_to": "participants",
        "requires": ["fs.write[calendar]"],
    },
    "cancel_calendar_event": {
        "control": ["event_id"],
        "requires": ["fs.write[calendar]"],
    },
    "send_email": {
        "control": ["recipients", "cc", "bcc"],
        "discloses_to": "recipients",
        "requires": ["net.egress[{recipients}]"],
    },
}

#: Mirrors CaMeL's `workspace.py`. Note `create_file`: writing a file the
#: user alone can see is not a disclosure, and CaMeL allows it unconditionally
#: for that reason. Sharing it is the act that needs a trusted address.
WORKSPACE: dict[str, Declaration] = {
    # Looking things up.
    "get_unread_emails": {},
    "get_sent_emails": {},
    "get_received_emails": {},
    "get_draft_emails": {},
    "search_emails": {},
    "search_contacts_by_name": {},
    "search_contacts_by_email": {},
    "get_current_day": {},
    "search_calendar_events": {},
    "get_day_calendar_events": {},
    "search_files_by_filename": {},
    "get_file_by_id": {},
    "list_files": {},
    "search_files": {},
    # The outbox.
    "send_email": {
        "control": ["recipients", "cc", "bcc"],
        "discloses_to": "recipients",
        "requires": ["net.egress[{recipients}]"],
    },
    "delete_email": {"control": ["email_id"], "requires": ["fs.write[inbox]"]},
    # The calendar.
    "create_calendar_event": {
        "control": ["participants"],
        "discloses_to": "participants",
        "requires": ["fs.write[calendar]"],
    },
    "cancel_calendar_event": {"control": ["event_id"], "requires": ["fs.write[calendar]"]},
    "reschedule_calendar_event": {
        "control": ["event_id"],
        "requires": ["fs.write[calendar]"],
    },
    "add_calendar_event_participants": {
        "control": ["event_id", "participants"],
        "discloses_to": "participants",
        "requires": ["fs.write[calendar]"],
    },
    # The drive.
    "create_file": {"requires": ["fs.write[drive]"]},
    "delete_file": {"control": ["file_id"], "requires": ["fs.write[drive]"]},
    "share_file": {
        "control": ["email"],
        "discloses_to": "email",
        "requires": ["net.egress[{email}]"],
    },
    "append_to_file": {"control": ["file_id"], "requires": ["fs.write[drive]"]},
}

#: Every suite this knows how to declare.
SUITES: dict[str, dict[str, Declaration]] = {
    "banking": BANKING,
    "slack": SLACK,
    "travel": TRAVEL,
    "workspace": WORKSPACE,
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
