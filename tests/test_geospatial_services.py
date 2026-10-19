"""Local-file OSM reverse geocoding and urban-heat-island services."""

from __future__ import annotations

import json

import pytest

from pyweatherenriched import ReverseGeocoder, UrbanHeatIsland

REVERSE_GEOJSON = {
    "type": "FeatureCollection",
    "features": [
        {
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [-74.006, 40.7128]},
            "properties": {
                "addr:postcode": "10001",
                "addr:city": "New York",
                "addr:state": "NY",
                "addr:country": "US",
            },
        }
    ],
}

UHI_GEOJSON = {
    "type": "FeatureCollection",
    "features": [
        {
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [
                    [
                        [-74.001, 40.001],
                        [-74.0, 40.001],
                        [-74.0, 40.0],
                        [-74.001, 40.0],
                        [-74.001, 40.001],
                    ]
                ],
            },
            "properties": {"building": "yes", "height": "20"},
        }
    ],
}


def _write_tile(directory, body):
    (directory / "40_-74.geojson").write_text(json.dumps(body))


def test_reverse_geocode_reads_fixture_tile(tmp_path):
    _write_tile(tmp_path, REVERSE_GEOJSON)
    geocoder = ReverseGeocoder(str(tmp_path))

    result = geocoder.reverse_geocode(40.7128, -74.0)

    assert result["postal_code"] == "10001"
    assert result["city"] == "New York"
    assert result["state"] == "NY"
    assert result["country_code"] == "US"
    assert result["source"] == "osm"
    assert geocoder.cache_stats() == (1, 5000)


def test_reverse_geocode_missing_tile_raises_instead_of_fabricating(tmp_path):
    geocoder = ReverseGeocoder(str(tmp_path))

    with pytest.raises(FileNotFoundError, match="40_-74.geojson"):
        geocoder.reverse_geocode(40.7128, -74.0)


def test_reverse_geocode_tile_without_postal_code_raises(tmp_path):
    _write_tile(
        tmp_path,
        {
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [-74.0, 40.0]},
                    "properties": {"addr:city": "No Postal Code"},
                }
            ],
        },
    )
    geocoder = ReverseGeocoder(str(tmp_path))

    with pytest.raises(RuntimeError, match="no postal code"):
        geocoder.reverse_geocode(40.0, -74.0)


def test_reverse_geocode_rejects_out_of_range_coordinates(tmp_path):
    geocoder = ReverseGeocoder(str(tmp_path))

    with pytest.raises(ValueError, match="latitude"):
        geocoder.reverse_geocode(91.0, 0.0)
    with pytest.raises(ValueError, match="longitude"):
        geocoder.reverse_geocode(0.0, 181.0)


def test_uhi_reads_fixture_building_footprints(tmp_path):
    _write_tile(tmp_path, UHI_GEOJSON)
    uhi = UrbanHeatIsland(str(tmp_path))

    result = uhi.get_uhi_data(40.0005, -74.0)

    assert result["building_density_percent"] == pytest.approx(0.95, abs=0.1)
    assert result["average_building_height_m"] == pytest.approx(20.0)
    assert result["uhi_effect_c"] > 0.7
    assert result["location_type"] == "rural"


def test_uhi_missing_tile_raises(tmp_path):
    uhi = UrbanHeatIsland(str(tmp_path))

    with pytest.raises(FileNotFoundError, match="40_-74.geojson"):
        uhi.get_uhi_data(40.0, -74.0)


def test_optional_layers_and_alt_providers_are_not_exposed():
    import pyweatherenriched

    for name in ("Vegetation", "Soil", "FloodRisk", "GoogleMapsGeocoder"):
        assert not hasattr(pyweatherenriched, name)
