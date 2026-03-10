use crate::client::{Disruption, SncfClient, format_iso8601};
use crate::output::{HumanReadable, Output};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DisruptionResult {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub application_periods: Vec<PeriodResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impacted_lines: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impacted_stops: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PeriodResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl HumanReadable for DisruptionResult {
    fn print_human(&self) {
        let severity_str = self.severity.as_deref().unwrap_or("unknown");
        let status_colored = match self.status.as_str() {
            "active" => self.status.red(),
            "past" => self.status.dimmed(),
            _ => self.status.yellow(),
        };

        println!(
            "{} [{}] {}",
            severity_str.bold(),
            status_colored,
            self.cause.as_deref().unwrap_or("")
        );

        if let Some(msg) = &self.message {
            // Truncate very long messages for human display
            let display = if msg.len() > 200 {
                format!("{}...", &msg[..200])
            } else {
                msg.clone()
            };
            println!("  {display}");
        }

        if !self.impacted_lines.is_empty() {
            println!("  Lines: {}", self.impacted_lines.join(", "));
        }

        if !self.application_periods.is_empty() {
            for p in &self.application_periods {
                let begin = p.begin.as_deref().unwrap_or("?");
                let end = p.end.as_deref().unwrap_or("?");
                println!("  Period: {} \u{2192} {}", begin, end);
            }
        }

        println!();
    }
}

fn disruption_to_result(d: Disruption, use_iso: bool) -> DisruptionResult {
    let severity = d.severity.as_ref().and_then(|s| s.name.clone());
    let effect = d.severity.as_ref().and_then(|s| s.effect.clone());

    let message = d.messages.iter().find_map(|m| m.text.clone());

    let fmt = |s: &str| -> String {
        if use_iso {
            format_iso8601(s)
        } else {
            s.to_string()
        }
    };

    let application_periods: Vec<PeriodResult> = d
        .application_periods
        .iter()
        .map(|p| PeriodResult {
            begin: p.begin.as_deref().map(&fmt),
            end: p.end.as_deref().map(&fmt),
        })
        .collect();

    let mut impacted_lines = Vec::new();
    let mut impacted_stops = Vec::new();

    for io in &d.impacted_objects {
        if let Some(pt) = &io.pt_object
            && let Some(name) = &pt.name
        {
            match pt.embedded_type.as_deref() {
                Some("line") => impacted_lines.push(name.clone()),
                Some("stop_area") | Some("stop_point") => impacted_stops.push(name.clone()),
                _ => {}
            }
        }
        for stop in &io.impacted_stops {
            if let Some(sp) = &stop.stop_point
                && let Some(name) = &sp.name
            {
                impacted_stops.push(name.clone());
            }
        }
    }

    impacted_stops.sort();
    impacted_stops.dedup();

    DisruptionResult {
        id: d.id,
        status: d.status,
        severity,
        effect,
        cause: d.cause,
        message,
        application_periods,
        impacted_lines,
        impacted_stops,
    }
}

pub async fn run(
    client: &SncfClient,
    output: &Output,
    station: Option<&str>,
    line: Option<&str>,
) -> Result<()> {
    // Resolve station name to ID if provided
    let station_id = match station {
        Some(name) => Some(client.resolve_station(name).await?),
        None => None,
    };

    let disruptions = client.get_disruptions(station_id.as_deref(), line).await?;

    let use_iso = output.is_json();
    let results: Vec<DisruptionResult> = disruptions
        .into_iter()
        .map(|d| disruption_to_result(d, use_iso))
        .collect();

    output.print_list(&results);
    Ok(())
}
