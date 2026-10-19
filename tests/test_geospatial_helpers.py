"""Exposed pure Rust geospatial helpers (no SRTM/OSM data required)."""

from __future__ import annotations

import pytest

from pyweatherenriched import lapse_rate_adjustment, terrain_roughness


def test_lapse_rate_adjustment():
    assert lapse_rate_adjustment(0.0) == 0.0
    assert lapse_rate_adjustment(1000.0) == pytest.approx(-6.5, abs=1e-5)


def test_terrain_roughness_buckets():
    assert terrain_roughness(100.0) == pytest.approx(0.1)
    assert terrain_roughness(1000.0) == pytest.approx(0.2)
    assert terrain_roughness(2500.0) == pytest.approx(0.5)
