//! Shared data types for weather enrichment.

use serde::{Deserialize, Serialize};

/// A single enriched (location, timestamp) -> weather observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnrichedData {
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub temperature: f64,
    pub humidity: f64,
    pub condition: String,
    pub timestamp: String,
}
