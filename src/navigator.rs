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
    pub files_walked: Vec<String>,
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
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
            .collect()
    }

    fn get_walker_config(&mut self) -> WalkerPatterns {
        let args = Args::parse();

        if let Some(config) = args.config {
            let file = File::open(config.as_str()).expect("file should open read only");
            let json: serde_json::Value =
                serde_json::from_reader(file).expect("JSON was not well-formatted");

            let base_function = json.get("functions").unwrap();
            let functions = base_function
                .clone()
                .as_object_mut()
                .unwrap()
                .iter_mut()
                .map(|(key, value)| {
                    let text = Some(value.get("text").unwrap().as_u64().unwrap() as usize);
                    let context = value.get("context").map(|c| c.as_u64().unwrap() as usize);
                    let plural = value.get("plural").map(|c| c.as_u64().unwrap() as usize);

                    (
                        key.to_string(),
                        Function {
                            text,
                            context,
                            plural,
                        },
                    )
                })
                .collect::<HashMap<String, Function>>();

            self.visitor.functions = Some(functions);

            let exclude = json
                .get("exclude")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect::<Vec<String>>();

            let include = json
                .get("include")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect::<Vec<String>>();

            let patterns = Vec::from_iter(include.iter().chain(exclude.iter()))
                .iter()
                .map(|s| s.to_string())
                .collect();

            WalkerPatterns {
                base: json
                    .get("base")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(|| panic!("Invalid or missing 'base' field in JSON")),
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

        let mut sorted_usage: Vec<(&String, &usize)> =
            self.visitor.stats.usage_breakdown.iter().collect();
        sorted_usage.sort_by(|a, b| a.0.cmp(b.0));
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
