use std::collections::{HashMap, HashSet};

use swc_core::ecma::ast::{Callee, CallExpr, Ident};
use swc_core::ecma::visit::{Visit, VisitWith};

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum GettextOption {
    Gettext,
    Ngettext,
    Pgettext,
    Npgettext,
}

#[derive(Debug)]
pub struct Message {
    text: String,
    text_plural: Option<String>,
    context: String,
    references: HashSet<String>,
}

#[derive(Debug)]
pub struct Stats {
    pub messages: usize,
    pub plural: usize,
    pub usages: usize,
    pub context: usize,
    pub files_parsed: usize,
    pub files_with_messages: usize,
    pub usage_breakdown: HashMap<GettextOption, usize>,
}

#[derive(Debug)]
pub struct Visitor {
    pub contexts: HashMap<String, HashMap<String, Message>>,
    pub current_file: String,
    pub visited_files_with_messages: HashSet<String>,
    pub stats: Stats,
}

impl Visitor {
    fn add_message(&mut self, sym: GettextOption, node: &CallExpr) {
        let message = match sym {
            GettextOption::Gettext => {
                let context = String::from("");

                if let Some(first_arg) = crate::get_argument(node.args.first()) {
                    let text = first_arg.value.to_string();
                    let text_plural = None;

                    Some(Message {
                        text,
                        text_plural,
                        context,
                        references: HashSet::new(),
                    })
                } else {
                    None
                }
            }
            GettextOption::Ngettext => {
                let context = String::from("");
                if let Some(first_arg) = crate::get_argument(node.args.first()) {
                    let text = first_arg.value.to_string();
                    let text_plural = Some(crate::get_argument(node.args.get(1)).unwrap().value.to_string());

                    Some(Message {
                        text,
                        text_plural,
                        context,
                        references: HashSet::new(),
                    })
                } else {
                    None
                }
            }
            GettextOption::Pgettext => {
                if let Some(first_arg) = crate::get_argument(node.args.first()) {
                    let context = first_arg.value.to_string();
                    let text = crate::get_argument(node.args.get(1)).unwrap().value.to_string();
                    let text_plural = None;

                    Some(Message {
                        text,
                        text_plural,
                        context,
                        references: HashSet::new(),
                    })
                } else {
                    None
                }
            }
            GettextOption::Npgettext => {
                if let Some(first_arg) = crate::get_argument(node.args.first()) {
                    let context = first_arg.value.to_string();
                    let text = crate::get_argument(node.args.get(1)).unwrap().value.to_string();
                    let text_plural = Some(crate::get_argument(node.args.get(2)).unwrap().value.to_string());
                    dbg!(&text);
                    Some(Message {
                        text,
                        text_plural,
                        context,
                        references: HashSet::new(),
                    })
                } else {
                    None
                }
            }
        };

        if let Some(msg) = message {
            self.stats.usages += 1;
            self.stats.usage_breakdown
                .entry(sym)
                .and_modify(|e| *e += 1)
                .or_insert(1);

            self.visited_files_with_messages.insert(self.current_file.clone());
            self.stats.files_with_messages = self.visited_files_with_messages.len();
            self.add_to_catalog(msg);
        }
    }

    fn add_to_catalog(&mut self, message: Message) {
        let Message {
            text,
            text_plural,
            context,
            ..
        } = message;

        if !self.contexts.contains_key(&context) {
            self.stats.context += 1;
        }

        let context_map = self.contexts
            .entry(context.clone())
            .or_insert_with(HashMap::new);

        if context_map.contains_key(&text) {
            let entry = context_map.get_mut(&text).unwrap();
            entry.references.insert(self.current_file.clone());
        } else {
            self.stats.messages += 1;
            if text_plural.is_some() {
                self.stats.plural += 1;
            }

            context_map.insert(text.clone(), Message {
                text,
                text_plural,
                context,
                references: HashSet::from([self.current_file.clone()]),
            });
        }
    }

    fn parse_gettext(&mut self, node: &CallExpr, ident: &Ident) {
        match ident.sym.to_string().as_str() {
            "gettext" => self.add_message(GettextOption::Gettext, node),
            "ngettext" => self.add_message(GettextOption::Ngettext, node),
            "pgettext" => self.add_message(GettextOption::Pgettext, node),
            "npgettext" => self.add_message(GettextOption::Npgettext, node),
            _ => {}
        };
    }
}

impl Visit for Visitor {
    fn visit_call_expr(&mut self, node: &CallExpr) {
        node.visit_children_with(self);

        if let Callee::Expr(callee_expr) = &node.callee {
            if let Some(ident) = callee_expr.as_member() {
                if let Some(ident) = ident.prop.as_ident() {
                    self.parse_gettext(node, ident);
                }
            }

            if let Some(ident) = callee_expr.as_opt_chain() {
                if let Some(ident) = ident.base.as_member() {
                    if let Some(ident) = ident.prop.as_ident() {
                        self.parse_gettext(node, ident);
                    }
                }
            }

            if let Some(ident) = callee_expr.as_ident() {
                self.parse_gettext(node, ident);
            }
        }
    }
}
