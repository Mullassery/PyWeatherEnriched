"""Example: PyWeatherEnriched MCP 2.0"""

import logging
import time

from pyweatherenriched import WeatherEnricher

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


def main():
    logger.info("=" * 60)
    logger.info("PyWeatherEnriched MCP 2.0 Example")
    logger.info("=" * 60)

    enricher = WeatherEnricher()

    logger.info("\n1. Starting MCP connector...")
    try:
        endpoint = enricher.start_mcp_connector(port=8769)
        logger.info(f"✓ MCP endpoint ready: {endpoint}")
    except Exception as e:
        logger.error(f"Failed: {e}")
        return

    logger.info("\n2. MCP Tools Available (10 total):")
    tools = [
        "get_enrichment_status",
        "query_enriched_data",
        "get_weather_events",
        "detect_weather_anomalies",
        "correlate_business_metric",
        "forecast_weather_impact",
        "export_enriched_data",
        "validate_enrichment_quality",
        "list_available_enrichments",
        "get_hyperlocal_weather",
    ]
    for i, tool in enumerate(tools, 1):
        logger.info(f"  {i}. {tool}")

    logger.info("\n3. Claude Can Now:")
    logger.info('  • "Show weather impact on revenue"')
    logger.info('  • "Find storms affecting our regions"')
    logger.info('  • "Forecast weather impact next week"')

    logger.info("\n   MCP is running! Press Ctrl+C to stop...")

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("\n\nStopping...")
        enricher.stop_mcp_connector()


if __name__ == "__main__":
    main()
