extern crate globwalk;
extern crate swc_common;
extern crate swc_ecma_parser;

use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
    sync::Lrc,
};
use swc_core::ecma::ast::{EsVersion, ExprOrSpread, Lit, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use crate::navigator::Navigator;
use crate::visitor::{Stats, Visitor};

mod navigator;
mod visitor;

pub fn get_argument(node: Option<&ExprOrSpread>) -> Option<&Str> {
    if let Lit::Str(singular) = node?.expr.as_lit()? {
        return Some(singular);
    }
    None
}

fn main() {
    let cm: Lrc<SourceMap> = Default::default();
    let config: TsConfig = Default::default();
    let syntax = {
        Syntax::Typescript(TsConfig {
            tsx: true,
            decorators: config.decorators,
            dts: config.dts,
            no_early_errors: config.no_early_errors,
            disallow_ambiguous_jsx_like: false,
        })
    };
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let mut navigator = Navigator {
        files_walked: vec![],
        visitor: Visitor {
            visited_files_with_messages: Default::default(),
            current_file: Default::default(),
            contexts: Default::default(),
            stats: Stats {
                messages: 0,
                plural: 0,
                usages: 0,
                context: 0,
                files_parsed: 0,
                files_with_messages: 0,
                usage_breakdown: Default::default(),
            },
        },
    };

    for file in navigator.build() {
        let source_file = cm.load_file(file.path()).unwrap();
        let mut parser = Parser::new_from(Lexer::new(
            syntax,
            EsVersion::latest(),
            StringInput::from(&*source_file),
            None,
        ));

        for e in parser.take_errors() {
            eprintln!("Error creating parser from");
            e.into_diagnostic(&handler).emit();
        }

        match parser.parse_module() {
            Ok(module) => {
                let path = file.path().to_str().unwrap().to_string();
                navigator.parse(&module, path);
            }
            Err(e) => {
                eprintln!("Error parsing module");
                e.into_diagnostic(&handler).emit();
            }
        };
    }
    navigator.output();
}

