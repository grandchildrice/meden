"""Explicit capability requirements and exact AgentDojo argument schemas."""

from __future__ import annotations

from typing import Any

BANKING = {
    "get_iban": {"requires": ["secret.read[own-iban]"]},
    "get_balance": {"requires": []},
    "get_most_recent_transactions": {"requires": []},
    "get_scheduled_transactions": {"requires": []},
    "get_user_info": {"requires": []},
    "read_file": {"requires": ["fs.read[{file_path}]"]},
    "send_money": {"requires": ["net.egress[{recipient}]"]},
    "schedule_transaction": {"requires": ["net.egress[{recipient}]"]},
    "update_scheduled_transaction": {"requires": ["net.egress[{recipient}]"]},
    "update_password": {"requires": ["secret.read[password]"]},
    "update_user_info": {"requires": ["fs.write[user-info]"]},
}

SLACK = {
    "get_channels": {"requires": []},
    "read_channel_messages": {"requires": ["fs.read[{channel}]"]},
    "read_inbox": {"requires": ["fs.read[inbox]"]},
    "get_users_in_channel": {"requires": []},
    "get_webpage": {"requires": ["net.egress[{url}]"]},
    "add_user_to_channel": {"requires": ["fs.write[{channel}]"]},
    "send_channel_message": {"requires": ["net.egress[{channel}]"]},
    "send_direct_message": {"requires": ["net.egress[{recipient}]"]},
    "invite_user_to_slack": {"requires": ["net.egress[{user_email}]"]},
    "remove_user_from_slack": {"requires": ["fs.write[workspace]"]},
    "post_webpage": {"requires": ["net.egress[{url}]"]},
}

TRAVEL = {
    "get_user_information": {"requires": []},
    "get_all_hotels_in_city": {"requires": []},
    "get_hotels_prices": {"requires": []},
    "get_rating_reviews_for_hotels": {"requires": []},
    "get_hotels_address": {"requires": []},
    "get_all_restaurants_in_city": {"requires": []},
    "get_cuisine_type_for_restaurants": {"requires": []},
    "get_restaurants_address": {"requires": []},
    "get_rating_reviews_for_restaurants": {"requires": []},
    "get_dietary_restrictions_for_all_restaurants": {"requires": []},
    "get_contact_information_for_restaurants": {"requires": []},
    "get_price_for_restaurants": {"requires": []},
    "check_restaurant_opening_hours": {"requires": []},
    "get_all_car_rental_companies_in_city": {"requires": []},
    "get_car_types_available": {"requires": []},
    "get_rating_reviews_for_car_rental": {"requires": []},
    "get_car_fuel_options": {"requires": []},
    "get_car_rental_address": {"requires": []},
    "get_car_price_per_day": {"requires": []},
    "get_flight_information": {"requires": []},
    "search_calendar_events": {"requires": []},
    "get_day_calendar_events": {"requires": []},
    "reserve_hotel": {"requires": ["net.egress[booking]"]},
    "reserve_car_rental": {"requires": ["net.egress[booking]"]},
    "reserve_restaurant": {"requires": ["net.egress[booking]"]},
    "create_calendar_event": {"requires": ["fs.write[calendar]"]},
    "cancel_calendar_event": {"requires": ["fs.write[calendar]"]},
    "send_email": {"requires": ["net.egress[{recipients}]"]},
}

WORKSPACE = {
    "get_unread_emails": {"requires": []},
    "get_sent_emails": {"requires": []},
    "get_received_emails": {"requires": []},
    "get_draft_emails": {"requires": []},
    "search_emails": {"requires": []},
    "search_contacts_by_name": {"requires": []},
    "search_contacts_by_email": {"requires": []},
    "get_current_day": {"requires": []},
    "search_calendar_events": {"requires": []},
    "get_day_calendar_events": {"requires": []},
    "search_files_by_filename": {"requires": []},
    "get_file_by_id": {"requires": []},
    "list_files": {"requires": []},
    "search_files": {"requires": []},
    "send_email": {"requires": ["net.egress[{recipients}]"]},
    "delete_email": {"requires": ["fs.write[inbox]"]},
    "create_calendar_event": {"requires": ["fs.write[calendar]"]},
    "cancel_calendar_event": {"requires": ["fs.write[calendar]"]},
    "reschedule_calendar_event": {"requires": ["fs.write[calendar]"]},
    "add_calendar_event_participants": {"requires": ["fs.write[calendar]"]},
    "create_file": {"requires": ["fs.write[drive]"]},
    "delete_file": {"requires": ["fs.write[drive]"]},
    "share_file": {"requires": ["net.egress[{email}]"]},
    "append_to_file": {"requires": ["fs.write[drive]"]},
}

SUITES = {"banking": BANKING, "slack": SLACK, "travel": TRAVEL, "workspace": WORKSPACE}


class UndeclaredTool(ValueError):
    """A runtime tool has no host declaration."""


def tool_specs(runtime: Any, declarations: dict) -> list[dict[str, Any]]:
    specs = []
    for name, tool in runtime.functions.items():
        if name not in declarations:
            raise UndeclaredTool(name)
        specs.append(
            {
                "name": name,
                "description": tool.description,
                "parameters_schema": tool.parameters.model_json_schema(),
                "requires": list(declarations[name].get("requires", [])),
            }
        )
    return specs


PERMISSIVE_POLICY = {
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
