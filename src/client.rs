//! SNCF API client using Navitia
//!
//! API documentation: https://doc.navitia.io/
//! SNCF API base: https://api.sncf.com/v1/

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const SNCF_API_BASE: &str = "https://api.sncf.com/v1";
const COVERAGE: &str = "sncf";

/// SNCF API client
pub struct SncfClient {
    client: Client,
    api_key: String,
}

// =============================================================================
// API Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PlacesResponse {
    pub places: Option<Vec<Place>>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct JourneysResponse {
    pub journeys: Option<Vec<Journey>>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct Journey {
    pub sections: Vec<Section>,
    pub departure_date_time: String,
    pub arrival_date_time: String,
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Section {
    #[serde(rename = "type")]
    pub section_type: String,
    #[serde(default)]
    pub departure_date_time: Option<String>,
    #[serde(default)]
    pub arrival_date_time: Option<String>,
    #[serde(default)]
    pub base_departure_date_time: Option<String>,
    #[serde(default)]
    pub base_arrival_date_time: Option<String>,
    #[serde(default)]
    pub data_freshness: Option<String>,
    #[serde(default)]
    pub display_informations: Option<SectionDisplayInfo>,
    #[serde(default)]
    pub from: Option<SectionPlace>,
    #[serde(default)]
    pub to: Option<SectionPlace>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionDisplayInfo {
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub commercial_mode: Option<String>,
    #[serde(default)]
    pub physical_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionPlace {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub stop_point: Option<SectionStopPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionStopPoint {
    #[serde(default)]
    pub platform_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub embedded_type: String,
    pub quality: u32,
    #[serde(default)]
    pub stop_area: Option<StopArea>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StopArea {
    pub id: String,
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub coord: Option<Coord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Coord {
    pub lon: String,
    pub lat: String,
}

// =============================================================================
// Client Implementation
// =============================================================================

impl SncfClient {
    /// Create a new client from SNCF_API_KEY environment variable
    pub fn from_env() -> Result<Self> {
        let api_key =
            std::env::var("SNCF_API_KEY").context("SNCF_API_KEY environment variable not set")?;
        Ok(Self::new(api_key))
    }

    /// Create a new client with explicit API key
    pub fn new(api_key: String) -> Self {
        let client = Client::new();
        Self { client, api_key }
    }

    /// Search for stations/stop areas by name
    pub async fn search_places(&self, query: &str, limit: u32) -> Result<Vec<Place>> {
        let url = format!(
            "{}/coverage/{}/places?q={}&type[]=stop_area&count={}",
            SNCF_API_BASE,
            COVERAGE,
            urlencoding::encode(query),
            limit
        );

        let response: PlacesResponse = self
            .client
            .get(&url)
            .basic_auth(&self.api_key, None::<&str>)
            .send()
            .await
            .context("Failed to send request")?
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error) = response.error {
            anyhow::bail!("API error: {} - {}", error.id, error.message);
        }

        Ok(response.places.unwrap_or_default())
    }

    /// Get journeys between two stations
    pub async fn get_journeys(
        &self,
        from_id: &str,
        to_id: &str,
        limit: u32,
        datetime: Option<String>,
    ) -> Result<Vec<Journey>> {
        let dt = datetime.unwrap_or_else(|| Utc::now().format("%Y%m%dT%H%M%S").to_string());
        let url = format!(
            "{}/coverage/{}/journeys?from={}&to={}&datetime={}&count={}&data_freshness=realtime",
            SNCF_API_BASE, COVERAGE, from_id, to_id, dt, limit
        );

        let response: JourneysResponse = self
            .client
            .get(&url)
            .basic_auth(&self.api_key, None::<&str>)
            .send()
            .await
            .context("Failed to send request")?
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error) = response.error {
            anyhow::bail!("API error: {} - {}", error.id, error.message);
        }

        Ok(response.journeys.unwrap_or_default())
    }

    /// Resolve a station name or ID to a stop_area ID.
    /// If the input starts with "stop_area:", it's returned as-is.
    /// Otherwise, search for the station and return the first match.
    pub async fn resolve_station(&self, name_or_id: &str) -> Result<String> {
        if name_or_id.starts_with("stop_area:") {
            return Ok(name_or_id.to_string());
        }
        let places = self.search_places(name_or_id, 1).await?;
        places
            .first()
            .map(|p| p.id.clone())
            .ok_or_else(|| anyhow::anyhow!("No station found for: {}", name_or_id))
    }
}

/// Parse SNCF datetime format (YYYYMMDDTHHMMSS) to chrono DateTime
pub fn parse_sncf_datetime(s: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S")
        .ok()
        .map(|dt| dt.and_utc())
}

/// Calculate delay in minutes between scheduled and actual time
pub fn calculate_delay_minutes(scheduled: &str, actual: &str) -> Option<i64> {
    let scheduled_dt = parse_sncf_datetime(scheduled)?;
    let actual_dt = parse_sncf_datetime(actual)?;
    Some((actual_dt - scheduled_dt).num_minutes())
}

/// Format SNCF datetime as HH:MM
pub fn format_time(s: &str) -> String {
    parse_sncf_datetime(s)
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sncf_datetime() {
        let dt = parse_sncf_datetime("20250201T143000").unwrap();
        assert_eq!(dt.format("%H:%M").to_string(), "14:30");
    }

    #[test]
    fn test_calculate_delay() {
        let delay = calculate_delay_minutes("20250201T143000", "20250201T143500").unwrap();
        assert_eq!(delay, 5);
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time("20250201T143000"), "14:30");
    }
}
