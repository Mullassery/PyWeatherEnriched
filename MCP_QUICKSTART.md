# PyWeatherEnriched MCP 2.0 Quick Start

> AI-native weather enrichment. Ask Claude to correlate business metrics with weather, forecast impact, find anomalies.

## Installation

```bash
pip install PyWeatherEnriched>=1.0
```

## Basic Usage

```python
from pyweatherenriched import WeatherEnricher

# Create enricher
enricher = WeatherEnricher()

# Enable MCP (starts on port 8769)
endpoint = enricher.start_mcp_connector()

# Claude can now:
# - "Which locations were affected by storms last week?"
# - "Correlate our sales with rainfall events"
# - "Show enrichment coverage by region"
# - "Forecast weather impact on revenue next week"
```

## 10 MCP Tools

1. `get_enrichment_status` — Coverage & data quality by location
2. `query_enriched_data` — Time-series weather metrics
3. `get_weather_events` — Storm, heat, cold, flood detection
4. `get_location_coverage` — Regional coverage gaps
5. `detect_weather_anomalies` — Find unusual patterns
6. `correlate_business_metric` — Link sales/traffic to weather
7. `forecast_weather_impact` — Predict impact on business
8. `export_enriched_data` — Multi-format export
9. `validate_enrichment_quality` — Quality assessment
10. `get_hyperlocal_weather` — Neighborhood-level precision

---

For full documentation, see [README.md](README.md)
