use crate::client::{calculate_delay_minutes, format_time, Journey, Section, SncfClient};
use crate::output::{HumanReadable, Output};
use anyhow::Result;
use chrono::{Local, NaiveTime};
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JourneyResult {
    pub train_type: String,
    pub departure: String,
    pub arrival: String,
    pub duration_minutes: u64,
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
    pub delay_minutes: Option<i64>,
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
            None => "—".dimmed().to_string(),
        };

        let changes = if self.changes == 0 {
            "direct".to_string()
        } else {
            format!("{} chg", self.changes)
        };

        println!(
            "{:<14} {} → {}  {}  {:<12} {}",
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

fn journey_to_result(journey: Journey) -> JourneyResult {
    let pt_sections: Vec<&Section> = journey
        .sections
        .iter()
        .filter(|s| s.section_type == "public_transport")
        .collect();

    let changes = pt_sections.len().saturating_sub(1) as u32;

    // Main train type: pick the longest section (by name length as proxy, or first)
    let main_section = pt_sections.first();
    let train_type = main_section
        .and_then(|s| s.display_informations.as_ref())
        .and_then(|d| d.commercial_mode.clone())
        .unwrap_or_default();

    // Max delay across all sections
    let delay_minutes = pt_sections
        .iter()
        .filter_map(|s| section_delay(s))
        .max();

    let duration_minutes = journey.duration.unwrap_or(0) / 60;

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
                departure: s.departure_date_time.clone().unwrap_or_default(),
                arrival: s.arrival_date_time.clone().unwrap_or_default(),
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
        departure: journey.departure_date_time,
        arrival: journey.arrival_date_time,
        duration_minutes,
        delay_minutes,
        changes,
        sections,
    }
}

pub async fn run(
    client: &SncfClient,
    output: &Output,
    from: &str,
    to: &str,
    limit: u32,
    at: Option<&str>,
) -> Result<()> {
    let from_id = client.resolve_station(from).await?;
    let to_id = client.resolve_station(to).await?;

    let datetime = at.map(|time_str| {
        let time = NaiveTime::parse_from_str(time_str, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::parse_from_str(time_str, "%H%M").unwrap());
        let today = Local::now().date_naive();
        today.and_time(time).format("%Y%m%dT%H%M%S").to_string()
    });

    let journeys = client
        .get_journeys(&from_id, &to_id, limit, datetime)
        .await?;

    let results: Vec<JourneyResult> = journeys.into_iter().map(journey_to_result).collect();

    output.print_list(&results);
    Ok(())
}
