"""Offline climate feature tests."""

from __future__ import annotations

import pandas as pd
from pyweatherenriched.features import (
    add_climate_normals,
    add_growing_degree_days,
    add_heat_index,
)


def _frame():
    return pd.DataFrame(
        {
            "timestamp": pd.date_range("2024-01-01", periods=40, freq="D", tz="UTC"),
            "location": ["A"] * 20 + ["B"] * 20,
            "temperature": [10.0] * 20 + [20.0] * 20,
            "humidity": [50.0] * 40,
        }
    )


def test_normals_are_per_group():
    out = add_climate_normals(_frame(), group_col="location")
    assert "temperature_normal" in out
    assert out.loc[out["location"] == "A", "temperature_anomaly"].abs().max() < 1e-9
    assert out.loc[out["location"] == "B", "temperature_anomaly"].abs().max() < 1e-9


def test_heat_index_and_gdd():
    out = add_heat_index(_frame())
    assert out["heat_index_c"].notna().all()
    out = add_growing_degree_days(out)
    assert (out["gdd_base10"] >= 0).all()
