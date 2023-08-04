use std::collections::HashMap;
use std::fs::File;

use clap::Parser;
use globwalk::{DirEntry, GlobWalkerBuilder};
use serde_json::Value;
use swc_core::ecma::ast::Module;
use swc_core::ecma::visit::Visit;

use crate::visitor::{Function, Visitor};

#[derive(Parser)]
struct Args {
    #[arg(short, long, required_unless_present("config"))]
    base: Option<String>,

    #[arg(short, long)]
    exclude: Vec<String>,

    #[arg(short, long)]
    include: Vec<String>,

    #[arg(short, long)]
    config: Option<String>,
}

pub struct Navigator {
    pub visitor: Visitor,
}

struct WalkerPatterns {
    base: String,
    patterns: Vec<String>,
}

impl Navigator {
    pub fn build(&mut self) -> Vec<DirEntry> {
        let WalkerPatterns { base, patterns } = self.get_walker_config();

        GlobWalkerBuilder::from_patterns(base, &patterns)
            .build()
            .expect("Failed to build GlobWalker")
            .filter_map(Result::ok)
            .collect()
    }

    fn get_walker_config(&mut self) -> WalkerPatterns {
        let args = Args::parse();

        if let Some(config) = args.config {
            let file = File::open(config.as_str()).expect("file should open read only");
            let json: serde_json::Value =
                serde_json::from_reader(file).expect("JSON was not well-formatted");

            let functions = json
                .get("functions")
                .expect("Missing 'functions' field in JSON")
                .as_object()
                .expect("'functions' field is not an object in JSON")
                .iter()
                .map(|(key, value)| {
                    (
                        key.to_string(),
                        Function {
                            text: Self::get_usize(value, "text"),
                            context: Self::get_usize(value, "context"),
                            plural: Self::get_usize(value, "plural"),
                        },
                    )
                })
                .collect();

            self.visitor.functions = Some(functions);

            let patterns = ["include", "exclude"]
                .iter()
                .flat_map(|key| Self::get_pattern_vec(&json, key))
                .collect();

            WalkerPatterns {
                base: json
                    .get("base")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .expect("Invalid or missing 'base' field in JSON"),
                patterns,
            }
        } else {
            let base = args.base.unwrap();
            let patterns = args.include.iter().chain(&args.exclude).cloned().collect();

            let functions = HashMap::from([
                (
                    "gettext".to_string(),
                    Function {
                        text: Some(0),
                        context: None,
                        plural: None,
                    },
                ),
                (
                    "ngettext".to_string(),
                    Function {
                        text: Some(0),
                        context: None,
                        plural: Some(1),
                    },
                ),
                (
                    "pgettext".to_string(),
                    Function {
                        text: Some(1),
                        context: Some(0),
                        plural: None,
                    },
                ),
                (
                    "npgettext".to_string(),
                    Function {
                        text: Some(1),
                        context: Some(0),
                        plural: Some(2),
                    },
                ),
            ]);

            println!("\n    Using default functions: {:?} \n", functions.keys());

            self.visitor.functions = Some(functions);

            WalkerPatterns { base, patterns }
        }
    }

    fn get_pattern_vec(json: &Value, key: &str) -> Vec<String> {
        json.get(key)
            .and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| panic!("Missing or invalid '{}' field in JSON", key))
    }

    fn get_usize(value: &Value, index: &str) -> Option<usize> {
        value.get(index).and_then(Value::as_u64).map(|v| v as usize)
    }

    pub fn parse(&mut self, module: &Module, path: String) {
        self.visitor.stats.files_parsed += 1;
        self.visitor.current_file = path;

        self.visitor.visit_module(module);
    }

    pub fn output(&self) {
        println!("    BEGIN TO EXTRACT:\n");
        println!("    {:?} messages extracted", self.visitor.stats.messages);
        println!("  -------------------------------");
        println!("    {:?} total usages", self.visitor.stats.usages);

        let mut sorted_usage = self
            .visitor
            .stats
            .usage_breakdown
            .iter()
            .collect::<Vec<_>>();
        sorted_usage.sort_by(|a, b| b.1.cmp(a.1));
        for (key, count) in &sorted_usage {
            println!("    ↳ {:<5} {:?} usages", count, key);
        }

        println!(
            "\n    {:?} files ({:?} with messages)",
            self.visitor.stats.files_parsed, self.visitor.stats.files_with_messages
        );
        println!("    {:?} message contexts", self.visitor.stats.context);
        println!("\n    EXTRACT FINISHED");
    }
}
