extern crate globwalk;
extern crate swc_common;
extern crate swc_ecma_parser;

use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_core::ecma::ast::{EsVersion, ExprOrSpread, Lit, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use crate::navigator::Navigator;
use crate::visitor::{Stats, Visitor};

mod navigator;
mod visitor;

pub fn get_argument(node: Option<&ExprOrSpread>) -> Option<&Str> {
    node?.expr.as_lit().and_then(|lit| match lit {
        Lit::Str(singular) => Some(singular),
        _ => None,
    })
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
        visitor: Visitor {
            visited_files_with_messages: Default::default(),
            current_file: Default::default(),
            contexts: Default::default(),
            functions: None,
            stats: Stats::default(),
        },
    };

    navigator.build().iter().for_each(|file| {
        let source_file = cm.load_file(file.path()).expect("Failed to load file");
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
    });

    navigator.output();
}
