use std::fs::File;
use clap::Parser;
use globwalk::{DirEntry, GlobWalkerBuilder};
use swc_core::ecma::ast::Module;
use swc_core::ecma::visit::Visit;

use crate::visitor::Visitor;

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

            let base = json.get("base").unwrap().as_str().unwrap();

            let exclude = json.get("exclude").unwrap().as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect::<Vec<String>>();

            let include = json.get("include").unwrap().as_array().unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect::<Vec<String>>();


            let patterns = Vec::from_iter(include.iter().chain(exclude.iter()))
                .iter()
                .map(|s| s.to_string())
                .collect();

            WalkerPatterns {
                base: base.to_string(),
                patterns,
            }
        } else {
            let base = args.base.unwrap();
            let patterns = Vec::from_iter(args.include.iter().chain(args.exclude.iter()))
                .iter()
                .map(|s| s.to_string())
                .collect();

            WalkerPatterns {
                base,
                patterns,
            }
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
        for (key, count) in self.visitor.stats.usage_breakdown.iter() {
            println!("    ↳ {:?} {:?} usages", count, format!("{:?}", key).to_lowercase());
        }
        println!(
            "\n    {:?} files ({:?} with messages)",
            self.visitor.stats.files_parsed,
            self.visitor.stats.files_with_messages
        );
        println!("    {:?} message contexts", self.visitor.stats.context);
        println!("\n    EXTRACT FINISHED");
    }
}
