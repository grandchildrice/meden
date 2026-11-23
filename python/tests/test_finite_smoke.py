"""The live trial's independent expected behavior rejects the overflow bug."""
from pathlib import Path

import pytest

from meden_checks.finite_smoke import independent_check


def test_independent_finite_oracle_checks_every_original_counter_input():
    examples = Path(__file__).resolve().parents[2] / "verification/examples"
    assert independent_check((examples / "finite-counter.model.json").read_text()) == 256
    with pytest.raises(AssertionError):
        independent_check((examples / "finite-counter-broken.model.json").read_text())
