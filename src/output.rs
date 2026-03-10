use crate::error::Error;
use colored::Colorize;
use serde::Serialize;
use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub struct Output {
    json: bool,
    pretty: bool,
    quiet: bool,
    fields: Vec<String>,
}

impl Output {
    pub fn new(json: Option<bool>, pretty: bool, quiet: bool, fields: Vec<String>) -> Self {
        let json = json.unwrap_or_else(|| !std::io::stdout().is_terminal());
        Self {
            json,
            pretty,
            quiet,
            fields,
        }
    }

    pub fn print<T: Serialize + HumanReadable>(&self, data: &T) {
        if self.json {
            self.print_json(data);
        } else if !self.quiet {
            data.print_human();
        }
    }

    pub fn print_list<T: Serialize + HumanReadable>(&self, items: &[T]) {
        if self.json {
            self.print_json(&items);
        } else if !self.quiet {
            for item in items {
                item.print_human();
            }
        }
    }

    fn print_json<T: Serialize>(&self, data: &T) {
        let value = serde_json::to_value(data).unwrap();
        let filtered = self.filter_fields(value);
        let output = if self.pretty {
            serde_json::to_string_pretty(&filtered).unwrap()
        } else {
            serde_json::to_string(&filtered).unwrap()
        };
        println!("{output}");
    }

    fn filter_fields(&self, value: serde_json::Value) -> serde_json::Value {
        if self.fields.is_empty() {
            return value;
        }
        match value {
            serde_json::Value::Object(map) => {
                let filtered: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .filter(|(k, _)| self.fields.iter().any(|f| f == k))
                    .collect();
                serde_json::Value::Object(filtered)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(|v| self.filter_fields(v)).collect())
            }
            other => other,
        }
    }

    pub fn error(&self, msg: &str) {
        eprintln!("{} {}", "\u{2717}".red(), msg);
    }

    pub fn error_structured(&self, err: &Error) {
        if self.json {
            let obj = serde_json::json!({
                "error": err.to_string(),
                "code": err.code(),
            });
            let output = if self.pretty {
                serde_json::to_string_pretty(&obj).unwrap()
            } else {
                serde_json::to_string(&obj).unwrap()
            };
            eprintln!("{output}");
        } else {
            self.error(&err.to_string());
        }
    }

    pub fn is_json(&self) -> bool {
        self.json
    }
}

pub trait HumanReadable {
    fn print_human(&self);
}
