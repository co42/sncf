use clap::{Parser, Subcommand};
use sncf::commands;
use sncf::{Output, SncfClient};

#[derive(Parser)]
#[command(name = "sncf", about = "SNCF train schedule CLI", version)]
struct Cli {
    /// Force JSON output
    #[arg(long, global = true)]
    json: bool,

    /// Force human output (override TTY auto-detect)
    #[arg(long, global = true)]
    no_json: bool,

    /// Filter output fields (comma-separated, JSON mode only)
    #[arg(long, global = true, value_delimiter = ',')]
    fields: Vec<String>,

    /// Suppress status messages
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn output(&self) -> Output {
        let json = if self.json {
            Some(true)
        } else if self.no_json {
            Some(false)
        } else {
            None
        };
        Output::new(json, self.quiet, self.fields.clone())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Search stations by name
    Search {
        /// Station name query
        query: String,
        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Next trains between two stations
    Next {
        /// Origin station (name or stop_area ID)
        from: String,
        /// Destination station (name or stop_area ID)
        to: String,
        /// Maximum results
        #[arg(long, default_value = "5")]
        limit: u32,
        /// Departure time (HH:MM), defaults to now
        #[arg(long)]
        at: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let output = cli.output();
    let client = SncfClient::from_env()?;

    let result = match cli.command {
        Commands::Search { query, limit } => {
            commands::search::run(&client, &output, &query, limit).await
        }
        Commands::Next {
            from,
            to,
            limit,
            at,
        } => commands::next::run(&client, &output, &from, &to, limit, at.as_deref()).await,
    };

    if let Err(e) = result {
        output.error(&e.to_string());
        std::process::exit(1);
    }

    Ok(())
}
