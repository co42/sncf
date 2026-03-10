use colored::Colorize;
use serde::Serialize;
use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub struct Output {
    json: bool,
    quiet: bool,
    fields: Vec<String>,
}

impl Output {
    pub fn new(json: Option<bool>, quiet: bool, fields: Vec<String>) -> Self {
        let json = json.unwrap_or_else(|| !std::io::stdout().is_terminal());
        Self {
            json,
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
        println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
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
        eprintln!("{} {}", "✗".red(), msg);
    }

    pub fn is_json(&self) -> bool {
        self.json
    }
}

pub trait HumanReadable {
    fn print_human(&self);
}
