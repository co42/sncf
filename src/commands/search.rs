use crate::client::{Place, SncfClient};
use crate::output::{HumanReadable, Output};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StationResult {
    pub id: String,
    pub name: String,
    pub label: String,
}

impl HumanReadable for StationResult {
    fn print_human(&self) {
        println!(
            "{}  {:<24}  {}",
            self.id.dimmed(),
            self.name.bold(),
            self.label
        );
    }
}

impl From<Place> for StationResult {
    fn from(place: Place) -> Self {
        let (name, label) = match &place.stop_area {
            Some(sa) => (sa.name.clone(), sa.label.clone()),
            None => (place.name.clone(), place.name.clone()),
        };
        Self {
            id: place.id,
            name,
            label,
        }
    }
}

pub async fn run(client: &SncfClient, output: &Output, query: &str, limit: u32) -> Result<()> {
    let places = client.search_places(query, limit).await?;
    let results: Vec<StationResult> = places.into_iter().map(StationResult::from).collect();
    output.print_list(&results);
    Ok(())
}
