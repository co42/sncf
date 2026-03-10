use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use sncf::commands;
use sncf::error::Error;
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

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,

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
        Output::new(json, self.pretty, self.quiet, self.fields.clone())
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
        /// Departure time (HH:MM or full datetime e.g. 2026-03-10T14:00)
        #[arg(long)]
        at: Option<String>,
        /// Departure date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },
    /// Show disruptions
    Disruptions {
        /// Filter by station name
        #[arg(long)]
        station: Option<String>,
        /// Filter by line name
        #[arg(long)]
        line: Option<String>,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let output = cli.output();

    // Completions don't need API key
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "sncf", &mut std::io::stdout());
        return;
    }

    let client = match SncfClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            let err = Error::Other(e);
            output.error_structured(&err);
            std::process::exit(err.exit_code());
        }
    };

    let result = match cli.command {
        Commands::Search { query, limit } => {
            commands::search::run(&client, &output, &query, limit).await
        }
        Commands::Next {
            from,
            to,
            limit,
            at,
            date,
        } => {
            commands::next::run(
                &client,
                &output,
                &from,
                &to,
                limit,
                at.as_deref(),
                date.as_deref(),
            )
            .await
        }
        Commands::Disruptions { station, line } => {
            commands::disruptions::run(&client, &output, station.as_deref(), line.as_deref()).await
        }
        Commands::Completions { .. } => unreachable!(),
    };

    if let Err(e) = result {
        // Try to convert anyhow::Error to our Error type for structured output
        let err = match e.downcast::<Error>() {
            Ok(typed_err) => typed_err,
            Err(generic) => Error::Other(generic),
        };
        output.error_structured(&err);
        std::process::exit(err.exit_code());
    }
}
