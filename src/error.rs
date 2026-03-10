use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("API error: {0}")]
    Api(String),

    #[error("No station found for query: {0}")]
    StationNotFound(String),

    #[error("SNCF_API_KEY environment variable not set")]
    MissingApiKey,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn code(&self) -> &str {
        match self {
            Error::Api(_) => "api",
            Error::StationNotFound(_) => "not_found",
            Error::MissingApiKey => "auth",
            Error::Config(_) => "config",
            Error::Http(_) => "api",
            Error::Json(_) => "api",
            Error::Other(_) => "generic",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Error::MissingApiKey => 2,
            Error::StationNotFound(_) => 3,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
