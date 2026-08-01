"""MCP Connector for PyWeatherEnriched - Weather Data Integration"""

import json
import logging
import subprocess
import tempfile
from abc import ABC, abstractmethod
from typing import Any, Dict, Optional

logger = logging.getLogger(__name__)

try:
    from statguardian._mcp_connector import BaseMCPConnector
except ImportError:
    class BaseMCPConnector(ABC):
        """Local fallback"""
        def __init__(self, project_name: str, port: int = 8765):
            self.project_name = project_name
            self.port = port
            self.dab_process: Optional[subprocess.Popen] = None
            self._ready = False

        @abstractmethod
        def get_mcp_tools(self) -> Dict[str, Any]:
            pass

        @abstractmethod
        def get_tool_handlers(self) -> Any:
            pass

        def start_mcp_connector(self) -> str:
            logger.info(f"Starting {self.project_name} MCP connector...")
            try:
                tools = self.get_mcp_tools()
                self.handler = self.get_tool_handlers()
                config = self._generate_dab_config(tools)
                config_path = self._write_temp_config(config)
                self._start_dab_subprocess(config_path)
                self._ready = True
                return f"http://localhost:{self.port}/mcp"
            except Exception as e:
                logger.error(f"Failed: {e}")
                raise

        def stop_mcp_connector(self):
            if self.dab_process:
                try:
                    self.dab_process.terminate()
                    self.dab_process.wait(timeout=5)
                except (subprocess.TimeoutExpired, OSError):
                    pass
                self._ready = False

        def _generate_dab_config(self, tools: Dict[str, Any]) -> Dict:
            return {
                "runtime": {"host": "0.0.0.0", "port": self.port, "cors": {"origins": ["*"]}},
                "entities": {k: {"source": k, "permissions": [{"actions": ["*"], "roles": ["*"]}]} for k in tools.keys()},
                "rest": {"enabled": True, "path": "/api"},
                "graphql": {"enabled": True, "path": "/graphql"},
                "mcp": {"enabled": True, "path": "/mcp"},
            }

        def _write_temp_config(self, config: Dict) -> str:
            with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
                json.dump(config, f)
                return f.name

        def _start_dab_subprocess(self, config_path: str):
            self.dab_process = subprocess.Popen(
                ["dab", "start", "--config", config_path],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

        def is_ready(self) -> bool:
            return self._ready


class WeatherEnricher:
    """Weather enrichment with MCP support"""

    def __init__(self):
        self.mcp_connector: Optional[Any] = None

    def get_status(self, location: str, start_date: str, end_date: str) -> Dict:
        return {"coverage": 85.0, "events": 3, "quality": 90.0, "updated": "2026-07-31T00:00:00Z"}

    def query(self, location: str, metric: str, time_range: str) -> Dict:
        return {"times": [], "values": [], "confidence": []}

    def get_events(self, location: str, event_type: str, time_range: str) -> Dict:
        return {"events": [], "severity": [], "areas": []}

    def get_coverage(self, region: str) -> Dict:
        return {"coverage": 85.0, "gaps": [], "updated": "2026-07-31T00:00:00Z"}

    def detect_anomalies(self, location: str, time_window_days: int) -> Dict:
        return {"anomalies": [], "severity": [], "area": ""}

    def correlate(self, metric_name: str, weather_event_type: str, location: str) -> Dict:
        return {"score": 0.5, "impact": 10.0, "confidence": 0.8, "examples": []}

    def forecast(self, location: str, forecast_days: int, business_metric: str) -> Dict:
        return {"events": [], "impact": {}, "confidence": 0.7}

    def export(self, location: str, date_range: str, format: str) -> Dict:
        return {"path": "", "rows": 0, "quality": {}}

    def validate(self, location: str, source_data: Dict, enriched_data: Dict) -> Dict:
        return {"score": 90.0, "gaps": [], "recommendations": []}

    def list_enrichments(self) -> Dict:
        return {"types": ["temperature", "rainfall", "storms"], "locations": [], "quality": {}}

    def get_hyperlocal(self, coordinates: tuple, time_range: str, resolution: str) -> Dict:
        return {"data": [], "confidence": 0.85}

    def start_mcp_connector(self, port: int = 8769) -> str:
        from pyweatherenriched._mcp_tools import PyWeatherEnrichedMCPHandler, PyWeatherEnrichedMCPTools
        self.mcp_connector = _MCPEnricherConnector(enricher=self, port=port)
        return self.mcp_connector.start_mcp_connector()

    def stop_mcp_connector(self):
        if self.mcp_connector:
            self.mcp_connector.stop_mcp_connector()


class _MCPEnricherConnector(BaseMCPConnector):
    """Internal connector"""

    def __init__(self, enricher: WeatherEnricher, port: int = 8769):
        super().__init__("PyWeatherEnriched", port=port)
        self.enricher = enricher

    def get_mcp_tools(self) -> Dict[str, Any]:
        from pyweatherenriched._mcp_tools import PyWeatherEnrichedMCPTools
        return PyWeatherEnrichedMCPTools.get_tools()

    def get_tool_handlers(self) -> Any:
        from pyweatherenriched._mcp_tools import PyWeatherEnrichedMCPHandler
        return PyWeatherEnrichedMCPHandler(self.enricher)
