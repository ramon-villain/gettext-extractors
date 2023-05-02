use std::collections::HashMap;
use std::fs::File;
use std::vec::IntoIter;

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

enum ConfigProperty {
    Include,
    Exclude,
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

        let a = match args.config {
            Some(config) => {
                let file = File::open(config.as_str()).expect("file should open read only");
                let json: Value =
                    serde_json::from_reader(file).expect("JSON was not well-formatted");

                let functions = json
                    .get("functions")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .iter()
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

                let exclude = Self::get_config_property(json.clone(), ConfigProperty::Exclude);
                let include = Self::get_config_property(json.clone(), ConfigProperty::Include);

                (
                    json.get("base").unwrap().as_str().unwrap().to_string(),
                    functions,
                    include.chain(exclude),
                )
            }
            None => {
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

                (
                    args.base.unwrap(),
                    functions,
                    args.include.into_iter().chain(args.exclude.into_iter()),
                )
            }
        };

        let patterns = Vec::from_iter(a.2.clone())
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.visitor.functions = Some(a.1);
        WalkerPatterns {
            base: a.0,
            patterns,
        }
    }

    fn get_config_property(json: Value, property: ConfigProperty) -> IntoIter<String> {
        let json = match property {
            ConfigProperty::Include => json.get("include"),
            ConfigProperty::Exclude => json.get("exclude"),
        };

        json.unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect::<Vec<String>>()
            .into_iter()
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
        for (key, count) in self.visitor.stats.usage_breakdown.iter() {
            println!("    ↳ {:?} {:?} usages", count, key);
        }
        println!(
            "\n    {:?} files ({:?} with messages)",
            self.visitor.stats.files_parsed, self.visitor.stats.files_with_messages
        );
        println!("    {:?} message contexts", self.visitor.stats.context);
        println!("\n    EXTRACT FINISHED");
    }
}
