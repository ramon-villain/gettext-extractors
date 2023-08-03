use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::{Callee, CallExpr, Ident};
use swc_core::ecma::visit::{Visit, VisitWith};

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
    pub usage_breakdown: HashMap<String, usize>,
}

#[derive(Debug)]
pub struct Visitor {
    pub contexts: HashMap<String, HashMap<String, Message>>,
    pub current_file: String,
    pub visited_files_with_messages: HashSet<String>,
    pub stats: Stats,
    pub functions: Option<HashMap<String, Function>>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub text: Option<usize>,
    pub context: Option<usize>,
    pub plural: Option<usize>,
}

impl Visitor {
    fn add_message(&mut self, key: String, fun: Function, node: &CallExpr) {
        let text = match fun.text {
            Some(text) => crate::get_argument(node.args.get(text)).map(|arg| arg.value.to_string()),
            None => None,
        };

        if text.is_none() {
            return;
        }

        let context = match fun.context {
            Some(context) => match crate::get_argument(node.args.get(context)) {
                Some(arg) => arg.value.to_string(),
                None => String::from(""),
            },
            None => String::from(""),
        };

        let text_plural = match fun.plural {
            Some(plural) => {
                crate::get_argument(node.args.get(plural)).map(|arg| arg.value.to_string())
            }
            None => None,
        };

        let message = Message {
            text: text.unwrap(),
            text_plural,
            context,
            references: HashSet::new(),
        };

        self.stats.usages += 1;
        self.stats
            .usage_breakdown
            .entry(key)
            .and_modify(|e| *e += 1)
            .or_insert(1);

        self.visited_files_with_messages
            .insert(self.current_file.clone());
        self.stats.files_with_messages = self.visited_files_with_messages.len();
        self.add_to_catalog(message);
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

        let context_map = self
            .contexts
            .entry(context.clone())
            .or_insert_with(HashMap::new);

        if context_map.contains_key(&text) {
            let entry = context_map.get_mut(&text).unwrap();
            entry.references.insert(self.current_file.clone());
        } else {
            self.stats.messages += 1;
            self.stats.plural += text_plural.is_some() as usize;

            context_map.insert(
                text.clone(),
                Message {
                    text,
                    text_plural,
                    context,
                    references: HashSet::from([self.current_file.clone()]),
                },
            );
        }
    }

    fn parse_gettext(&mut self, node: &CallExpr, ident: &Ident) {
        if let Some(functions) = &self.functions {
            if let Some(function) = functions.get(ident.sym.as_ref()) {
                self.add_message(
                    ident.sym.to_string(),
                    Function {
                        text: function.text,
                        context: function.context,
                        plural: function.plural,
                    },
                    node,
                );
            }
        }
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
