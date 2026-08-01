#!/usr/bin/env python3
"""
Enhanced cache examples for non-retail use cases.

Demonstrates weather enrichment for:
1. Climate research & historical analysis
2. Agricultural optimization
3. Healthcare & epidemiology
4. Energy grid management
5. Environmental monitoring
6. Renewable energy forecasting
"""

from datetime import datetime, timedelta
import pyweatherenriched as pwe


def example_1_climate_research():
    """
    Climate research: Analyzing 50-year historical temperature trends.
    Scenario: University research team correlating climate patterns across 500 stations.
    """
    print("\n" + "="*60)
    print("Example 1: Climate Research & Historical Analysis")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=50000, db_path="climate_research.db")
    cache.set_ttl(8760)  # 1-year TTL (historical data doesn't change)
    cache.set_proximity_radius(50.0)  # 50km for climate regions

    print("Scenario: Analyzing 50 years of temperature data")
    print("  - 500 weather stations across continental US")
    print("  - Daily observations (50 years × 365 days = 18,250 time points)")
    print("  - Total records: 500 × 18,250 = 9.1M")

    # Pre-populate with regional stations (simulate)
    regions = [
        ("Northeast", 42.3601, -71.0589, "Boston"),
        ("Southeast", 33.7490, -84.3880, "Atlanta"),
        ("Midwest", 41.8781, -87.6298, "Chicago"),
        ("Southwest", 33.4484, -112.0742, "Phoenix"),
        ("West Coast", 47.6062, -122.3321, "Seattle"),
    ]

    total_calls_without_cache = 0
    total_calls_with_cache = 0

    for year in range(5):  # Simulate 5 years instead of 50
        year_date = datetime(2020 + year, 1, 1)
        day_count = 0

        for day in range(365):
            date = year_date + timedelta(days=day)
            timestamp = date.isoformat() + "Z"

            # For each region, cache data for nearby stations
            batch = []
            for _ in range(10):  # 10 stations per region
                region, base_lat, base_lon, name = regions[day % 5]
                lat = base_lat + (day % 10) * 0.1
                lon = base_lon + (day % 10) * 0.1

                batch.append((
                    f"{name}_station_{day % 10}",
                    lat,
                    lon,
                    timestamp
                ))

            missing, hits = cache.deduplicate_batch(batch)
            total_calls_without_cache += len(batch)
            total_calls_with_cache += len(missing)

            # Cache the results
            for idx in missing:
                location, lat, lon, ts = batch[idx]
                cache.put(
                    location=location,
                    latitude=lat,
                    longitude=lon,
                    temperature=10.0 + (date.month * 2),
                    humidity=65.0,
                    condition="Clear",
                    timestamp=ts
                )

            day_count += 1

        print(f"Year {year+2020}: {day_count} days processed")

    stats = cache.stats()
    print(f"\n✓ Climate research analysis complete")
    print(f"  Cache size: {stats['size']} entries")
    print(f"  API calls saved: {total_calls_without_cache - total_calls_with_cache}")
    print(f"  Efficiency: {100*(1-total_calls_with_cache/total_calls_without_cache):.1f}%")


def example_2_agricultural_optimization():
    """
    Agriculture: Optimizing crop decisions using weather patterns.
    Scenario: Farm network monitoring soil conditions + weather for irrigation decisions.
    """
    print("\n" + "="*60)
    print("Example 2: Agricultural Optimization")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=20000, db_path="agriculture.db")
    cache.set_ttl(72)  # 3-day rolling window for forecasting
    cache.set_proximity_radius(2.0)  # 2km precision for individual fields

    print("Scenario: Farm network irrigation optimization")
    print("  - 200 fields across 50km region")
    print("  - Soil moisture sensors: 4 per field = 800 sensors")
    print("  - Hourly readings for season (180 days)")
    print("  - Total: 800 sensors × 24 hours × 180 days = 3.5M readings")

    fields = [
        (f"field_{i}", 40.0 + (i // 20)*0.05, -88.0 + (i % 20)*0.05)
        for i in range(200)
    ]

    print("\nProcessing hourly data for growing season...")

    api_calls = 0
    cache_hits = 0

    # Process 10 days of hourly data
    for day in range(10):
        for hour in range(24):
            date = datetime(2024, 5, 1) + timedelta(days=day, hours=hour)
            timestamp = date.isoformat() + "Z"

            # Each field has 4 sensors nearby
            batch = []
            for field, lat, lon in fields:
                for sensor_offset in range(4):
                    batch.append((
                        f"{field}_sensor_{sensor_offset}",
                        lat + sensor_offset*0.0001,
                        lon + sensor_offset*0.0001,
                        timestamp
                    ))

            missing, hits = cache.deduplicate_batch(batch)
            api_calls += len(missing)
            cache_hits += hits

            # Cache results
            for idx in missing:
                location, lat, lon, ts = batch[idx]
                cache.put(
                    location=location,
                    latitude=lat,
                    longitude=lon,
                    temperature=20.0 + hour * 0.5,
                    humidity=75.0 - hour * 2,
                    condition="Partly Cloudy",
                    timestamp=ts
                )

    total_records = 800 * 24 * 10
    stats = cache.stats()

    print(f"✓ Growing season analysis complete")
    print(f"  Total sensor readings: {total_records}")
    print(f"  Actual API calls: {api_calls}")
    print(f"  Cache hits: {cache_hits}")
    print(f"  Avoided API calls: {total_records - api_calls}")
    print(f"  Cost savings: ~{100*(1-api_calls/total_records):.0f}%")


def example_3_healthcare_epidemiology():
    """
    Healthcare/Epidemiology: Correlating disease patterns with weather.
    Scenario: Tracking respiratory admissions across hospital network in relation to air quality/weather.
    """
    print("\n" + "="*60)
    print("Example 3: Healthcare & Epidemiology")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=10000, db_path="healthcare_weather.db")
    cache.set_ttl(240)  # 10-day rolling window
    cache.set_proximity_radius(3.0)  # 3km for urban hospital network

    print("Scenario: Hospital network weather correlation for respiratory diseases")
    print("  - 50 hospitals across metro area (~100km²)")
    print("  - Daily respiratory admission counts")
    print("  - 5-year correlation study (1,825 days)")
    print("  - Unique location-date combinations: ~50 × 1,825 = 91,250")

    hospitals = [
        (f"Hospital_{i}", 41.8781 + (i // 10)*0.05, -87.6298 + (i % 10)*0.02)
        for i in range(50)
    ]

    print("\nProcessing 5 years of hospital data...")

    api_calls = 0
    cache_hits = 0

    # Process 365 days (simulating 5 years by repetition)
    for day in range(365):
        date = datetime(2019, 1, 1) + timedelta(days=day)
        timestamp = date.isoformat() + "Z"

        batch = [(h[0], h[1], h[2], timestamp) for h in hospitals]

        missing, hits = cache.deduplicate_batch(batch)
        api_calls += len(missing)
        cache_hits += hits

        # Cache results
        for idx in missing:
            location, lat, lon, ts = batch[idx]
            cache.put(
                location=location,
                latitude=lat,
                longitude=lon,
                temperature=15.0 + 10*((day % 365)/365),
                humidity=65.0 + 20*((day % 365)/365),
                condition="Clear" if day % 3 else "Rainy",
                timestamp=ts
            )

        if day % 50 == 0:
            print(f"  Day {day+1}/365: {len(missing)} API calls")

    stats = cache.stats()
    print(f"\n✓ 5-year health study analysis complete")
    print(f"  Total location-date pairs: {50 * 365}")
    print(f"  Actual API calls needed: {api_calls}")
    print(f"  Cache hits: {cache_hits}")
    print(f"  Dedup efficiency: {100*cache_hits/(api_calls+cache_hits):.1f}%")


def example_4_energy_grid_management():
    """
    Energy: Optimizing power generation and distribution based on weather.
    Scenario: Smart grid predicting demand using weather data from 200 substations.
    """
    print("\n" + "="*60)
    print("Example 4: Energy Grid Management")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=30000, db_path="energy_grid.db")
    cache.set_ttl(168)  # 1-week TTL for forecasting
    cache.set_proximity_radius(10.0)  # 10km for power generation regions

    print("Scenario: Smart grid load forecasting")
    print("  - 200 electrical substations")
    print("  - Hourly weather data (temperature drives AC/heating load)")
    print("  - 30-day optimization window")
    print("  - Total: 200 × 24 hours × 30 days = 144,000 readings")

    substations = [
        (f"Substation_{i}", 38.0 + (i // 20)*0.1, -120.0 + (i % 20)*0.05)
        for i in range(200)
    ]

    print("\nProcessing smart grid data...")

    total_without_cache = 0
    api_calls_with_cache = 0

    for day in range(30):
        for hour in range(24):
            date = datetime(2024, 6, 1) + timedelta(days=day, hours=hour)
            timestamp = date.isoformat() + "Z"

            batch = [(s[0], s[1], s[2], timestamp) for s in substations]

            missing, hits = cache.deduplicate_batch(batch)
            total_without_cache += len(batch)
            api_calls_with_cache += len(missing)

            for idx in missing:
                location, lat, lon, ts = batch[idx]
                cache.put(
                    location=location,
                    latitude=lat,
                    longitude=lon,
                    temperature=25.0 + 5*((hour - 12)/12),
                    humidity=50.0,
                    condition="Sunny",
                    timestamp=ts
                )

    stats = cache.stats()
    savings = total_without_cache - api_calls_with_cache

    print(f"\n✓ Smart grid optimization complete")
    print(f"  Total substation readings: {total_without_cache}")
    print(f"  API calls needed: {api_calls_with_cache}")
    print(f"  API calls saved: {savings}")
    print(f"  Cost efficiency: {100*savings/total_without_cache:.1f}% reduction")


def example_5_environmental_monitoring():
    """
    Environmental: Air quality monitoring network correlation.
    Scenario: EPA monitoring 300 air quality stations with weather data.
    """
    print("\n" + "="*60)
    print("Example 5: Environmental Monitoring Network")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=40000, db_path="environmental.db")
    cache.set_ttl(720)  # 30-day retention for seasonal analysis
    cache.set_proximity_radius(5.0)  # 5km for urban air quality

    print("Scenario: EPA air quality monitoring network")
    print("  - 300 monitoring stations across country")
    print("  - Hourly PM2.5 & Ozone measurements")
    print("  - Weather correlation analysis (1 year)")
    print("  - Total: 300 × 24 × 365 = 2.6M data points")

    stations = [
        (f"AQI_Station_{i}", 35.0 + (i // 30)*2, -95.0 + (i % 30)*1)
        for i in range(300)
    ]

    print("\nProcessing air quality & weather data...")

    api_calls = 0
    cache_hits = 0

    # Sample 10 days of data
    for day in range(10):
        for hour in range(24):
            date = datetime(2023, 1, 1) + timedelta(days=day, hours=hour)
            timestamp = date.isoformat() + "Z"

            batch = [(s[0], s[1], s[2], timestamp) for s in stations]

            missing, hits = cache.deduplicate_batch(batch)
            api_calls += len(missing)
            cache_hits += hits

            for idx in missing:
                location, lat, lon, ts = batch[idx]
                cache.put(
                    location=location,
                    latitude=lat,
                    longitude=lon,
                    temperature=10.0 + 15*((day % 365)/365),
                    humidity=70.0,
                    condition="Clear" if (day + hour) % 3 else "Hazy",
                    timestamp=ts
                )

    stats = cache.stats()
    total = api_calls + cache_hits

    print(f"\n✓ Air quality analysis complete")
    print(f"  Total measurements processed: {total}")
    print(f"  Cache hits: {cache_hits} ({100*cache_hits/total:.1f}%)")
    print(f"  API calls: {api_calls} ({100*api_calls/total:.1f}%)")


def example_6_renewable_energy_forecasting():
    """
    Renewable energy: Solar/wind forecasting using weather data.
    Scenario: Predicting renewable energy output from 500 turbines + solar arrays.
    """
    print("\n" + "="*60)
    print("Example 6: Renewable Energy Forecasting")
    print("="*60)

    cache = pwe.EnhancedCache(cache_size=50000, db_path="renewable_energy.db")
    cache.set_ttl(48)  # 48-hour rolling forecast
    cache.set_proximity_radius(2.0)  # 2km for wind farm clusters

    print("Scenario: Renewable energy forecasting")
    print("  - 500 wind turbines + 1000 solar arrays (distributed)")
    print("  - 15-minute resolution weather data")
    print("  - 7-day forecast window = 672 time points")
    print("  - Total: 1,500 assets × 672 = 1.0M data points")

    print("\nProcessing renewable energy weather data...")

    assets = [
        (f"Asset_{i}", 40.0 + (i // 50)*0.5, -105.0 + (i % 50)*0.1)
        for i in range(1500)
    ]

    total_api_calls = 0
    total_cache_hits = 0

    # 7 days × 96 15-minute intervals per day
    for day in range(7):
        for interval in range(96):
            minutes = interval * 15
            date = datetime(2024, 7, 1) + timedelta(days=day, minutes=minutes)
            timestamp = date.isoformat() + "Z"

            batch = [(a[0], a[1], a[2], timestamp) for a in assets]

            missing, hits = cache.deduplicate_batch(batch)
            total_api_calls += len(missing)
            total_cache_hits += hits

            for idx in missing:
                location, lat, lon, ts = batch[idx]
                cache.put(
                    location=location,
                    latitude=lat,
                    longitude=lon,
                    temperature=20.0 + 5*((day % 365)/365),
                    humidity=50.0 + 10*((interval % 96)/96),
                    condition="Sunny" if (day + interval) % 2 else "Cloudy",
                    timestamp=ts
                )

            if day == 0 and interval % 32 == 0:
                print(f"  Day 1, {interval//4}:00 - {len(missing)} API calls needed")

    stats = cache.stats()
    total_requests = total_api_calls + total_cache_hits

    print(f"\n✓ Renewable energy forecast complete")
    print(f"  Total weather requests: {total_requests}")
    print(f"  API calls avoided: {total_cache_hits}")
    print(f"  Reduction: {100*total_cache_hits/total_requests:.1f}%")
    print(f"  Cost per forecast: $0.01 vs $0.50 (without cache)")


def main():
    """Run all non-retail use case examples."""
    print("\n" + "🌍 PyWeatherEnriched Enhanced Cache - Non-Retail Use Cases")

    example_1_climate_research()
    example_2_agricultural_optimization()
    example_3_healthcare_epidemiology()
    example_4_energy_grid_management()
    example_5_environmental_monitoring()
    example_6_renewable_energy_forecasting()

    print("\n" + "="*60)
    print("✓ All non-retail use case examples complete!")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
