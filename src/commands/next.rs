use crate::client::{
    Journey, Section, SncfClient, calculate_delay_minutes, format_iso8601, format_time,
};
use crate::output::{HumanReadable, Output};
use anyhow::Result;
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JourneyResult {
    pub train_type: String,
    pub departure: String,
    pub arrival: String,
    pub duration_minutes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_minutes: Option<i64>,
    pub changes: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<SectionSummary>,
}

#[derive(Debug, Serialize)]
pub struct SectionSummary {
    pub mode: String,
    pub code: String,
    pub from: String,
    pub to: String,
    pub departure: String,
    pub arrival: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

impl HumanReadable for JourneyResult {
    fn print_human(&self) {
        let dep = format_time(&self.departure);
        let arr = format_time(&self.arrival);

        let hours = self.duration_minutes / 60;
        let mins = self.duration_minutes % 60;
        let duration = format!("{hours}h{mins:02}");

        let delay = match self.delay_minutes {
            Some(d) if d > 0 => format!("+{d} min").red().to_string(),
            Some(_) => "on time".green().to_string(),
            None => "\u{2014}".dimmed().to_string(),
        };

        let changes = if self.changes == 0 {
            "direct".to_string()
        } else {
            format!("{} chg", self.changes)
        };

        println!(
            "{:<14} {} \u{2192} {}  {}  {:<12} {}",
            self.train_type.bold(),
            dep,
            arr,
            duration.dimmed(),
            delay,
            changes.dimmed()
        );
    }
}

fn section_delay(s: &Section) -> Option<i64> {
    match (
        s.base_departure_date_time.as_deref(),
        s.departure_date_time.as_deref(),
    ) {
        (Some(base), Some(actual)) => calculate_delay_minutes(base, actual),
        _ => None,
    }
}

fn journey_to_result(journey: Journey, use_iso: bool) -> JourneyResult {
    let pt_sections: Vec<&Section> = journey
        .sections
        .iter()
        .filter(|s| s.section_type == "public_transport")
        .collect();

    let changes = pt_sections.len().saturating_sub(1) as u32;

    let main_section = pt_sections.first();
    let train_type = main_section
        .and_then(|s| s.display_informations.as_ref())
        .and_then(|d| d.commercial_mode.clone())
        .unwrap_or_default();

    let delay_minutes = pt_sections.iter().filter_map(|s| section_delay(s)).max();
    let duration_minutes = journey.duration.unwrap_or(0) / 60;

    let fmt = |s: &str| -> String {
        if use_iso {
            format_iso8601(s)
        } else {
            s.to_string()
        }
    };

    let sections: Vec<SectionSummary> = journey
        .sections
        .iter()
        .filter(|s| s.section_type == "public_transport")
        .map(|s| {
            let info = s.display_informations.as_ref();
            SectionSummary {
                mode: info
                    .and_then(|i| i.commercial_mode.clone())
                    .unwrap_or_default(),
                code: info.and_then(|i| i.code.clone()).unwrap_or_default(),
                from: s
                    .from
                    .as_ref()
                    .and_then(|f| f.name.clone())
                    .unwrap_or_default(),
                to: s
                    .to
                    .as_ref()
                    .and_then(|t| t.name.clone())
                    .unwrap_or_default(),
                departure: s
                    .departure_date_time
                    .as_deref()
                    .map(&fmt)
                    .unwrap_or_default(),
                arrival: s.arrival_date_time.as_deref().map(&fmt).unwrap_or_default(),
                delay_minutes: section_delay(s),
                platform: s
                    .from
                    .as_ref()
                    .and_then(|f| f.stop_point.as_ref())
                    .and_then(|sp| sp.platform_code.clone()),
            }
        })
        .collect();

    JourneyResult {
        train_type,
        departure: fmt(&journey.departure_date_time),
        arrival: fmt(&journey.arrival_date_time),
        duration_minutes,
        delay_minutes,
        changes,
        sections,
    }
}

/// Parse --at value which can be HH:MM or full ISO datetime
fn parse_at(at_str: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(at_str, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(at_str, "%H%M"))
        .ok()
}

/// Parse --at when it's a full datetime (e.g. 2026-03-10T14:00)
fn parse_at_datetime(at_str: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M:%S"))
        .ok()
}

pub async fn run(
    client: &SncfClient,
    output: &Output,
    from: &str,
    to: &str,
    limit: u32,
    at: Option<&str>,
    date: Option<&str>,
) -> Result<()> {
    let from_id = client.resolve_station(from).await?;
    let to_id = client.resolve_station(to).await?;

    let datetime = build_datetime(at, date);

    let journeys = client
        .get_journeys(&from_id, &to_id, limit, datetime)
        .await?;

    let use_iso = output.is_json();
    let results: Vec<JourneyResult> = journeys
        .into_iter()
        .map(|j| journey_to_result(j, use_iso))
        .collect();

    output.print_list(&results);
    Ok(())
}

fn build_datetime(at: Option<&str>, date: Option<&str>) -> Option<String> {
    match (at, date) {
        (Some(at_str), _) => {
            // Try full datetime first
            if let Some(ndt) = parse_at_datetime(at_str) {
                return Some(ndt.format("%Y%m%dT%H%M%S").to_string());
            }
            // HH:MM with optional date
            let time = parse_at(at_str).unwrap_or_else(|| Local::now().time());
            let day = date
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .unwrap_or_else(|| Local::now().date_naive());
            Some(day.and_time(time).format("%Y%m%dT%H%M%S").to_string())
        }
        (None, Some(date_str)) => {
            let day = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
            let now = Local::now();
            let time = if day == now.date_naive() {
                now.time()
            } else {
                NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            };
            Some(day.and_time(time).format("%Y%m%dT%H%M%S").to_string())
        }
        (None, None) => None,
    }
}
