use crate::client::{calculate_delay_minutes, format_time, SncfClient};
use crate::output::{HumanReadable, Output};
use anyhow::Result;
use chrono::{Local, NaiveTime};
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TrainResult {
    pub train_type: String,
    pub code: String,
    pub departure: String,
    pub arrival: String,
    pub delay_minutes: Option<i64>,
    pub platform: Option<String>,
    pub direction: Option<String>,
    pub data_freshness: Option<String>,
}

impl HumanReadable for TrainResult {
    fn print_human(&self) {
        let label = if self.code.is_empty() {
            self.train_type.clone()
        } else {
            format!("{} {}", self.train_type, self.code)
        };

        let dep = format_time(&self.departure);
        let arr = format_time(&self.arrival);

        let delay = match self.delay_minutes {
            Some(d) if d > 0 => format!("+{d} min").red().to_string(),
            Some(_) => "on time".green().to_string(),
            None => "—".dimmed().to_string(),
        };

        let platform = self
            .platform
            .as_deref()
            .filter(|p| !p.is_empty())
            .map(|p| format!("platform {p}"))
            .unwrap_or_else(|| "—".to_string());

        println!(
            "{:<16} {} → {}  {:<12} {}",
            label.bold(),
            dep,
            arr,
            delay,
            platform.dimmed()
        );
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

    let journeys = client.get_journeys(&from_id, &to_id, limit, datetime).await?;

    let results: Vec<TrainResult> = journeys
        .into_iter()
        .flat_map(|j| {
            j.sections
                .into_iter()
                .filter(|s| s.section_type == "public_transport")
                .map(|s| {
                    let info = s.display_informations.as_ref();
                    let delay = match (
                        s.base_departure_date_time.as_deref(),
                        s.departure_date_time.as_deref(),
                    ) {
                        (Some(base), Some(actual)) => calculate_delay_minutes(base, actual),
                        _ => None,
                    };
                    let platform = s
                        .from
                        .as_ref()
                        .and_then(|f| f.stop_point.as_ref())
                        .and_then(|sp| sp.platform_code.clone());

                    TrainResult {
                        train_type: info
                            .and_then(|i| i.commercial_mode.clone())
                            .unwrap_or_default(),
                        code: info.and_then(|i| i.code.clone()).unwrap_or_default(),
                        departure: s.departure_date_time.unwrap_or_default(),
                        arrival: s.arrival_date_time.unwrap_or_default(),
                        delay_minutes: delay,
                        platform,
                        direction: info.and_then(|i| i.direction.clone()),
                        data_freshness: s.data_freshness,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    output.print_list(&results);
    Ok(())
}
