use clap::Parser;
use globwalk::{DirEntry, GlobWalkerBuilder};
use swc_core::ecma::ast::Module;
use swc_core::ecma::visit::Visit;

use crate::visitor::Visitor;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    base: String,

    #[arg(short, long)]
    exclude: Vec<String>,

    #[arg(short, long)]
    include: Vec<String>,
}

pub struct Navigator {
    pub visitor: Visitor,
    pub files_walked: Vec<String>,
}

impl Navigator {
    pub fn build(&mut self) -> Vec<DirEntry> {
        let args = Args::parse();
        let patterns: Vec<&String> = Vec::from_iter(args.include.iter().chain(args.exclude.iter()));

        GlobWalkerBuilder::from_patterns(&args.base, &patterns)
            .build()
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
            .collect()
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
