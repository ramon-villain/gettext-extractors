extern crate globwalk;
extern crate swc_common;
extern crate swc_ecma_parser;

use clap::Parser as ClapParser;
use globwalk::{DirEntry, GlobWalkerBuilder};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

#[derive(Debug)]
struct Content {
    text: String,
    text_plural: Option<String>,
}

#[derive(Debug)]
struct Message {
    content: Content,
    context: HashMap<String, HashSet<String>>,
}

struct TaskVisitor {
    path: String,
    messages: BTreeMap<String, Message>,
    files: BTreeSet<String>,
    usage: HashMap<String, i32>,
}

impl TaskVisitor {
    fn new() -> Self {
        Self {
            path: String::new(),
            messages: BTreeMap::new(),
            files: BTreeSet::new(),
            usage: HashMap::new(),
        }
    }

    fn push_to_string_vec(&mut self, ident: &Ident, n: &CallExpr) {
        let sym = ident.sym.to_string();
        let asd: Option<bool> = match sym.as_str() {
            "gettext" => {
                if let Some(Lit::Str(singular)) = n.args.first().and_then(|a| a.expr.as_lit())
                {
                    let text = singular.value.to_string();
                    self.messages
                        .entry(text.clone())
                        .or_insert(Message {
                            content: Content {
                                text,
                                text_plural: None,
                            },
                            context: HashMap::new(),
                        })
                        .context
                        .entry(String::from(""))
                        .or_insert_with(HashSet::new)
                        .insert(self.path.clone());
                    Some(true)
                } else {
                    None
                }
            }
            "ngettext" => {
                if let Some(Lit::Str(singular)) = n.args.first().and_then(|a| a.expr.as_lit())
                {
                    if let Some(Lit::Str(plural)) =
                        n.args.iter().next().and_then(|a| a.expr.as_lit())
                    {
                        let text = singular.value.to_string();

                        self.messages
                            .entry(text.clone())
                            .or_insert(Message {
                                content: Content {
                                    text,
                                    text_plural: Some(plural.value.to_string()),
                                },
                                context: HashMap::new(),
                            })
                            .context
                            .entry(String::from(""))
                            .or_insert_with(HashSet::new)
                            .insert(self.path.clone());
                        Some(true)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "pgettext" => {
                if let Some(Lit::Str(singular)) = n.args.first().and_then(|a| a.expr.as_lit())
                {
                    if let Some(Lit::Str(context)) =
                        n.args.iter().next().and_then(|a| a.expr.as_lit())
                    {
                        let text = singular.value.to_string();

                        self.messages
                            .entry(text.clone())
                            .or_insert(Message {
                                content: Content {
                                    text,
                                    text_plural: None,
                                },
                                context: HashMap::new(),
                            })
                            .context
                            .entry(context.value.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(self.path.clone());
                    }
                }
                Some(true)
            }
            "npgettext" => {
                let args = &n.args.iter();
                let mut args_cloned = args.clone();
                let context = args_cloned.next().unwrap();
                let singular = args_cloned.next().unwrap();
                let plural = args_cloned.next().unwrap();

                if singular.expr.as_lit().is_none() {
                    return;
                }

                if let Lit::Str(context) = context.expr.as_lit().unwrap() {
                    if let Lit::Str(singular) = singular.expr.as_lit().unwrap() {
                        if let Lit::Str(plural) = plural.expr.as_lit().unwrap() {
                            let text = singular.value.to_string();

                            self.messages
                                .entry(text.clone())
                                .or_insert(Message {
                                    content: Content {
                                        text,
                                        text_plural: Some(plural.value.to_string()),
                                    },
                                    context: HashMap::new(),
                                })
                                .context
                                .entry(context.value.to_string())
                                .or_insert_with(HashSet::new)
                                .insert(self.path.clone());
                        }
                    }
                }
                Some(true)
            }
            _ => None,
        };
        if asd.is_some() {
            self.files.insert(self.path.clone());
            *self.usage.entry(sym).or_insert(0) += 1;
        }
    }
}

impl Visit for TaskVisitor {
    fn visit_call_expr(&mut self, n: &CallExpr) {
        n.visit_children_with(self);

        if let Some(callee_expr) = n.callee.as_expr() {
            if let Some(ident) = callee_expr.as_member() {
                if let Some(ident) = ident.prop.as_ident() {
                    self.push_to_string_vec(ident, n);
                }
            }

            if let Some(ident) = callee_expr.as_opt_chain() {
                if let Some(ident) = ident.base.as_member() {
                    if let Some(ident) = ident.prop.as_ident() {
                        self.push_to_string_vec(ident, n);
                    }
                }
            }

            if let Some(ident) = callee_expr.as_ident() {
                self.push_to_string_vec(ident, n);
            }
        }
    }
}

#[derive(ClapParser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    base: String,

    #[arg(short, long)]
    exclude: Vec<String>,

    #[arg(short, long)]
    include: Vec<String>,
}

fn main() {
    let mut visitor = TaskVisitor::new();

    let cm: Lrc<SourceMap> = Default::default();
    let syntax = {
        let config: TsConfig = Default::default();
        Syntax::Typescript(TsConfig {
            tsx: true,
            decorators: config.decorators,
            dts: config.dts,
            no_early_errors: config.no_early_errors,
            disallow_ambiguous_jsx_like: false,
        })
    };
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let walker = walker();

    for img in walker.iter() {
        let source_file = cm.load_file(img.path()).unwrap();
        let lexer = Lexer::new(
            syntax,
            EsVersion::latest(),
            StringInput::from(&*source_file),
            None,
        );
        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            eprintln!("Error creating parser from");
            e.into_diagnostic(&handler).emit();
        }

        visitor.path = img.path().to_str().unwrap().to_string();
        let parsed = parser.parse_module().unwrap();

        visitor.visit_module(&parsed);
    }
    print_message(walker.len(), &mut visitor);
}

fn walker() -> Vec<DirEntry> {
    let args = Args::parse();
    let patterns:Vec<&String> = Vec::from_iter(args.include.iter().chain(args.exclude.iter()));

    GlobWalkerBuilder::from_patterns(&args.base, &patterns)
        .build()
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap())
        .collect::<Vec<DirEntry>>()
}

fn print_message(total_files: usize, visitor: &mut TaskVisitor) {
    let unique_context = visitor
        .messages
        .values()
        .flat_map(|x| x.context.keys().collect::<Vec<&String>>())
        .collect::<HashSet<&String>>()
        .len();

    let messages_len = visitor
        .messages
        .values()
        .map(|x| x.context.len())
        .sum::<usize>();

    dbg!(&visitor.messages.keys());

    println!("    BEGIN TO EXTRACT:\n");
    println!("    {:?} messages extracted", messages_len);
    println!("  -------------------------------");
    println!("    {:?} total usages", visitor.usage.values().sum::<i32>());
    for (key, count) in visitor.usage.iter() {
        println!("    ↳ {:?} {:?} usages", count, key);
    }
    println!(
        "\n    {:?} files ({:?} with messages)",
        total_files,
        visitor.files.len()
    );
    println!("    {:?} message contexts", unique_context);
    println!("\n    EXTRACT FINISHED");
}
