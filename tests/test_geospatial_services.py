"""Local-file OSM reverse geocoding and urban-heat-island services."""

from __future__ import annotations

import json

import pytest

from pyweatherenriched import (
    FloodRiskLayer,
    ReverseGeocoder,
    SoilLayer,
    UrbanHeatIsland,
    VegetationLayer,
)

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


def test_optional_layers_read_local_geojson_tiles(tmp_path):
    (tmp_path / "40_-74.geojson").write_text(
        json.dumps(
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "geometry": {"type": "Point", "coordinates": [-74.0, 40.7]},
                        "properties": {
                            "ndvi": 0.72,
                            "vegetation_type": "forest",
                            "cooling_effect_c": 2.1,
                            "texture": "loam",
                            "water_holding_capacity_mm": 180.0,
                            "ph": 6.5,
                            "risk_score": 0.42,
                            "risk_category": "moderate",
                            "factors": {"rainfall": 0.4},
                        },
                    }
                ],
            }
        )
    )

    vegetation = VegetationLayer(str(tmp_path), "{lat}_{lon}.geojson").get_data(40.7, -74.0)
    assert vegetation["ndvi"] == pytest.approx(0.72)
    assert vegetation["vegetation_type"] == "forest"

    soil = SoilLayer(str(tmp_path), "{lat}_{lon}.geojson").get_data(40.7, -74.0)
    assert soil["texture"] == "loam"
    assert soil["water_holding_capacity_mm"] == pytest.approx(180.0)
    assert soil["ph"] == pytest.approx(6.5)

    flood = FloodRiskLayer(str(tmp_path), "{lat}_{lon}.geojson").get_data(40.7, -74.0)
    assert flood["risk_category"] == "moderate"
    assert flood["factors"]["rainfall"] == pytest.approx(0.4)


def test_missing_optional_tile_is_an_error(tmp_path):
    with pytest.raises(FileNotFoundError):
        VegetationLayer(str(tmp_path), "{lat}_{lon}.geojson").get_data(40.7, -74.0)
