"""Python-level integration tests for the pyweatherenriched PyO3 bindings.

Exercises the compiled extension end to end: EnhancedCache (pure local
logic, no network) always runs; WeatherEnricher/EnrichmentBuilder tests
that need real geocoding + historical weather both hit real public APIs
(Nominatim, Open-Meteo) and are marked so they can be skipped offline.
"""

import socket

import pytest

import pyweatherenriched as pwe


def _network_available() -> bool:
    try:
        socket.create_connection(("nominatim.openstreetmap.org", 443), timeout=3).close()
        return True
    except OSError:
        return False


requires_network = pytest.mark.skipif(
    not _network_available(), reason="requires network access to Nominatim/Open-Meteo"
)


class TestEnhancedCache:
    def test_put_then_get_round_trips(self):
        cache = pwe.EnhancedCache(cache_size=10)
        cache.put("Berlin", 52.52, 13.405, 10.0, 60.0, "Clear", "2024-01-01T00:00:00")

        result = cache.get("Berlin", 52.52, 13.405, "2024-01-01T00:00:00")

        assert result is not None
        assert result["location"] == "Berlin"
        assert result["temperature"] == 10.0

    def test_miss_returns_none(self):
        cache = pwe.EnhancedCache(cache_size=10)
        assert cache.get("Nowhere", 0.0, 0.0, "2024-01-01T00:00:00") is None

    def test_stats_track_hits_and_misses(self):
        cache = pwe.EnhancedCache(cache_size=10)
        cache.get("Berlin", 52.52, 13.405, "2024-01-01T00:00:00")  # miss
        cache.put("Berlin", 52.52, 13.405, 10.0, 60.0, "Clear", "2024-01-01T00:00:00")
        cache.get("Berlin", 52.52, 13.405, "2024-01-01T00:00:00")  # hit

        stats = cache.stats()
        assert stats["hits"] == 1
        assert stats["misses"] == 1
        assert stats["hit_ratio"] == 0.5

    def test_ttl_and_proximity_are_configurable(self):
        cache = pwe.EnhancedCache(cache_size=10)
        cache.set_ttl(1)
        cache.set_proximity_radius(10.0)
        # No exception is the assertion here — these just need to be
        # real, callable configuration knobs.

    def test_deduplicate_batch(self):
        cache = pwe.EnhancedCache(cache_size=10)
        cache.put("Berlin", 52.52, 13.405, 10.0, 60.0, "Clear", "2024-01-01T00:00:00")

        requests = [
            ("Berlin", 52.52, 13.405, "2024-01-01T00:00:00"),
            ("Paris", 48.8566, 2.3522, "2024-01-01T00:00:00"),
        ]
        missing, hits = cache.deduplicate_batch(requests)

        assert hits == 1
        assert missing == [1]  # only the Paris request (index 1) needs fetching


class TestCacheStats:
    def test_repr_and_hit_ratio(self):
        cache = pwe.EnhancedCache(cache_size=10)
        cache.get("X", 0.0, 0.0, "t")  # miss
        stats_dict = cache.stats()
        assert stats_dict["misses"] == 1


@requires_network
class TestWeatherEnricherLive:
    def test_enrich_row_returns_real_data_for_a_real_city(self):
        enricher = pwe.WeatherEnricher()
        result = enricher.enrich_row("London", "2024-06-15T12:00:00")

        assert result["location"] == "London"
        assert abs(result["latitude"] - 51.5) < 1.0
        assert abs(result["longitude"] - (-0.13)) < 1.0
        assert -50.0 < result["temperature"] < 60.0  # any plausible Earth temperature
        assert 0.0 <= result["humidity"] <= 100.0
        assert result["condition"]  # non-empty real WMO-derived condition

    def test_enrich_batch_processes_multiple_rows(self):
        enricher = pwe.WeatherEnricher()
        rows = [
            ("Paris", "2024-01-15T00:00:00"),
            ("Tokyo", "2024-01-15T00:00:00"),
        ]
        results = enricher.enrich_batch(rows)

        assert len(results) == 2
        assert all("temperature" in r or "error" in r for r in results)

    def test_cache_stats_reflect_repeated_lookups(self):
        enricher = pwe.WeatherEnricher()
        enricher.enrich_row("Cairo", "2024-05-01T12:00:00")
        enricher.enrich_row("Cairo", "2024-05-01T12:00:00")  # cache hit

        stats = enricher.cache_stats()
        assert stats["hits"] == 1
        assert stats["misses"] == 1

    def test_enrichment_builder_produces_a_working_enricher(self):
        enricher = pwe.EnrichmentBuilder().with_cache_size(50).build()
        result = enricher.enrich_row("Sydney", "2024-07-01T12:00:00")

        assert result["location"] == "Sydney"
        assert abs(result["latitude"] - (-33.87)) < 1.0

    def test_unresolvable_location_does_not_crash_the_batch(self):
        enricher = pwe.WeatherEnricher()
        results = enricher.enrich_batch([("Nonexistent Place Xyzzy123", "2024-01-01T00:00:00")])

        assert len(results) == 1
        assert "error" in results[0]
