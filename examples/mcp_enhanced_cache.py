#!/usr/bin/env python3
"""
Enhanced cache examples for PyWeatherEnriched.

Demonstrates:
1. Basic multi-tier caching
2. Geospatial clustering
3. Date range queries
4. Batch deduplication
5. Performance monitoring
"""

from datetime import datetime, timedelta
import pyweatherenriched as pwe


def example_1_basic_caching():
    """Basic memory + persistent caching."""
    print("\n" + "="*60)
    print("Example 1: Basic Multi-Tier Caching")
    print("="*60)

    # Create cache with both memory and persistent storage
    cache = pwe.EnhancedCache(cache_size=1000, db_path="weather_cache.db")

    # Cache some weather data
    cache.put(
        location="New York",
        latitude=40.7128,
        longitude=-74.0060,
        temperature=15.2,
        humidity=65.0,
        condition="Partly Cloudy",
        timestamp="2024-01-15T12:00:00Z"
    )

    # Retrieve from cache (memory tier)
    result = cache.get(
        location="New York",
        latitude=40.7128,
        longitude=-74.0060,
        timestamp="2024-01-15T12:00:00Z"
    )

    if result:
        print(f"✓ Cache hit: {result['location']} - {result['condition']}")
        print(f"  Temperature: {result['temperature']}°C")

    stats = cache.stats()
    print(f"\nCache stats: {stats['hits']} hits, {stats['misses']} misses")


def example_2_geospatial_clustering():
    """Intelligent nearby location matching."""
    print("\n" + "="*60)
    print("Example 2: Geospatial Clustering (Nearby Locations)")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=5000, db_path="weather_geo.db")

    # Set proximity radius to 10km
    cache.set_proximity_radius(10.0)

    # Cache weather for Times Square
    cache.put(
        location="Times Square",
        latitude=40.7580,
        longitude=-73.9855,
        temperature=14.5,
        humidity=68.0,
        condition="Sunny",
        timestamp="2024-01-15T12:00:00Z"
    )

    # Query for nearby location (Herald Square, ~1.5km away)
    nearby_result = cache.get(
        location="Herald Square",
        latitude=40.7505,
        longitude=-73.9865,
        timestamp="2024-01-15T12:00:00Z"
    )

    if nearby_result:
        print("✓ Proximity match found!")
        print(f"  Original: Times Square ({40.7580}, {-73.9855})")
        print(f"  Queried:  Herald Square ({40.7505}, {-73.9865})")
        print(f"  Distance: ~1.5km")
        print(f"  Reused weather: {nearby_result['condition']}")

    # Query for distant location (not in proximity)
    cache.get(
        location="Washington DC",
        latitude=38.9072,
        longitude=-77.0369,
        timestamp="2024-01-15T12:00:00Z"
    )

    stats = cache.stats()
    print(f"\nProximity hits: {stats['proximity_hits']}")


def example_3_batch_deduplication():
    """Identify unique requests before API calls."""
    print("\n" + "="*60)
    print("Example 3: Batch Deduplication")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=10000, db_path="weather_batch.db")

    # Pre-populate cache with common locations
    common_locations = [
        ("New York", 40.7128, -74.0060),
        ("Los Angeles", 34.0522, -118.2437),
        ("Chicago", 41.8781, -87.6298),
    ]

    for name, lat, lon in common_locations:
        cache.put(
            location=name,
            latitude=lat,
            longitude=lon,
            temperature=20.0 + lat/40,
            humidity=60.0,
            condition="Cloudy",
            timestamp="2024-01-15T12:00:00Z"
        )

    # Create batch of 1000 records with many duplicates
    batch = []
    for _ in range(250):
        batch.append(("New York", 40.7128, -74.0060, "2024-01-15T12:00:00Z"))
        batch.append(("Los Angeles", 34.0522, -118.2437, "2024-01-15T12:00:00Z"))
        batch.append(("Chicago", 41.8781, -87.6298, "2024-01-15T12:00:00Z"))
        batch.append(("Boston", 42.3601, -71.0589, "2024-01-15T12:00:00Z"))

    print(f"Processing batch of {len(batch)} records...")

    missing_indices, cache_hits = cache.deduplicate_batch(batch)

    print(f"✓ Cache hits: {cache_hits}")
    print(f"✓ Unique requests needing API: {len(missing_indices)}")
    print(f"✓ Deduplication savings: {cache_hits}/{len(batch)} = {100*cache_hits/len(batch):.1f}%")


def example_4_date_range_caching():
    """Query weather across date ranges."""
    print("\n" + "="*60)
    print("Example 4: Date Range Caching")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=5000, db_path="weather_dates.db")
    cache.set_ttl(240)  # 10-day TTL

    # Pre-populate cache with 7 days of data for NYC
    base_date = datetime(2024, 1, 15)
    for day in range(7):
        date = base_date + timedelta(days=day)
        timestamp = date.isoformat() + "Z"

        cache.put(
            location="New York",
            latitude=40.7128,
            longitude=-74.0060,
            temperature=10.0 + day,
            humidity=60.0 + day,
            condition="Cloudy" if day % 2 else "Sunny",
            timestamp=timestamp
        )

    print("✓ Cached 7 days of weather data for NYC")

    # Query for entire date range
    # Note: This would use get_range() API (to be called)
    stats = cache.stats()
    print(f"\nCache size: {stats['size']} entries")
    print(f"Hit ratio: {stats['hit_ratio']:.1%}")


def example_5_monitoring_and_stats():
    """Monitor cache performance."""
    print("\n" + "="*60)
    print("Example 5: Cache Performance Monitoring")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=1000, db_path="weather_monitor.db")

    # Simulate workload
    locations = [
        ("NYC", 40.7128, -74.0060),
        ("LA", 34.0522, -118.2437),
        ("CHI", 41.8781, -87.6298),
    ]

    print("Warming cache...")
    for i in range(30):  # 3 locations × 10 timestamps
        location, lat, lon = locations[i % 3]
        timestamp = (datetime(2024, 1, 15) + timedelta(hours=i)).isoformat() + "Z"

        cache.put(
            location=location,
            latitude=lat,
            longitude=lon,
            temperature=15.0 + i * 0.5,
            humidity=60.0,
            condition="Clear",
            timestamp=timestamp
        )

    print("Querying cache...")
    for i in range(50):  # Repeated queries
        location, lat, lon = locations[i % 3]
        timestamp = (datetime(2024, 1, 15) + timedelta(hours=i % 10)).isoformat() + "Z"

        cache.get(
            location=location,
            latitude=lat,
            longitude=lon,
            timestamp=timestamp
        )

    # Print statistics
    stats = cache.stats()
    print("\n📊 Cache Statistics:")
    print(f"  Total hits: {stats['hits']}")
    print(f"  Total misses: {stats['misses']}")
    print(f"  Hit ratio: {stats['hit_ratio']:.1%}")
    print(f"  Current size: {stats['size']} entries")
    print(f"  Proximity hits: {stats['proximity_hits']}")
    print(f"  Dedup savings: {stats['deduplication_saves']}")


def example_6_multi_day_enrichment():
    """Real-world scenario: enriching delivery data over 30 days."""
    print("\n" + "="*60)
    print("Example 6: Multi-Day Enrichment (Real-world Scenario)")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=10000, db_path="weather_30day.db")
    cache.set_ttl(240)  # 10-day rolling window
    cache.set_proximity_radius(5.0)  # 5km for urban areas

    # Simulate 100 delivery locations for 30 days
    delivery_locations = [
        (f"loc_{i}", 40.7128 + i*0.01, -74.0060 + i*0.01)
        for i in range(100)
    ]

    total_api_calls = 0

    for day in range(30):
        date = datetime(2024, 1, 1) + timedelta(days=day)
        timestamp = date.isoformat() + "Z"

        # Create batch for the day (100 locations × 10 records each)
        batch = []
        for location, lat, lon in delivery_locations:
            for j in range(10):
                batch.append((
                    location,
                    lat + j*0.001,
                    lon + j*0.001,
                    timestamp
                ))

        # Check what needs API calls
        missing, hits = cache.deduplicate_batch(batch)

        # Simulate API calls
        for idx in missing:
            location, lat, lon, ts = batch[idx]
            cache.put(
                location=location,
                latitude=lat,
                longitude=lon,
                temperature=15.0 + day,
                humidity=65.0,
                condition="Partly Cloudy",
                timestamp=ts
            )

        total_api_calls += len(missing)

        if day % 10 == 0:
            print(f"Day {day+1}: {len(missing)} API calls (cache hits: {hits})")

    stats = cache.stats()
    print(f"\n✓ 30-day enrichment complete")
    print(f"  Total API calls: {total_api_calls}")
    print(f"  Without cache: {30 * 1000} calls")
    print(f"  Savings: {100 * (1 - total_api_calls/(30*1000)):.1f}%")


def main():
    """Run all examples."""
    print("\n" + "🌤️  PyWeatherEnriched Enhanced Cache Examples")

    example_1_basic_caching()
    example_2_geospatial_clustering()
    example_3_batch_deduplication()
    example_4_date_range_caching()
    example_5_monitoring_and_stats()
    example_6_multi_day_enrichment()

    print("\n" + "="*60)
    print("✓ All examples complete!")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
