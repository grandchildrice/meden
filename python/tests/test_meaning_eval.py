"""Evaluation denominators retain abstentions and infrastructure failures."""

from meden_checks.meaning_eval import metrics


def test_false_accept_denominator_includes_unanswered_negative_cases():
    rows = [
        {"expected": "mismatch", "actual": "match"},
        {"expected": "mismatch", "actual": "unclear"},
        {"expected": "mismatch", "actual": "error"},
        {"expected": "match", "actual": "mismatch"},
        {"expected": "match", "actual": "match"},
    ]
    result = metrics(rows)
    assert result["false_accept_rate"] == 1 / 3
    assert result["coverage"] == 3 / 5
    assert result["false_reject"] == 1
    assert result["error"] == 1 and result["unclear"] == 1
    assert result["correct"] == 1 and result["total"] == 5
