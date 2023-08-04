use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::{CallExpr, Callee, Ident};
use swc_core::ecma::visit::{Visit, VisitWith};

#[derive(Debug)]
pub struct Message {
    text: String,
    text_plural: Option<String>,
    context: String,
    references: HashSet<String>,
}

#[derive(Debug, Default)]
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

fn string_from_option(node: &CallExpr, opt: Option<usize>) -> Option<String> {
    opt.and_then(|i| crate::get_argument(node.args.get(i)).map(|arg| arg.value.to_string()))
}

impl Visitor {
    fn add_message(&mut self, key: String, fun: Function, node: &CallExpr) {
        if let Some(text) = string_from_option(node, fun.text) {
            self.stats.usages += 1;
            *self.stats.usage_breakdown.entry(key).or_insert(0) += 1;

            self.visited_files_with_messages
                .insert(self.current_file.clone());
            self.stats.files_with_messages = self.visited_files_with_messages.len();
            self.add_to_catalog(Message {
                text,
                text_plural: string_from_option(node, fun.plural),
                context: string_from_option(node, fun.context).unwrap_or_default(),
                references: HashSet::default(),
            });
        }
    }

    fn add_to_catalog(&mut self, message: Message) {
        let Message {
            text,
            text_plural,
            context,
            ..
        } = message;
        self.stats.context += !self.contexts.contains_key(&context) as usize;

        let context_map = self
            .contexts
            .entry(context.clone())
            .or_insert_with(HashMap::new);

        let entry = context_map.entry(text.clone()).or_insert_with(|| {
            self.stats.messages += 1;
            self.stats.plural += text_plural.is_some() as usize;

            Message {
                text,
                text_plural,
                context,
                references: HashSet::from([self.current_file.clone()]),
            }
        });

        entry.references.insert(self.current_file.clone());
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
            if let Some(ident) = callee_expr
                .as_member()
                .and_then(|ident| ident.prop.as_ident())
                .or_else(|| {
                    callee_expr.as_opt_chain().and_then(|ident| {
                        ident
                            .base
                            .as_member()
                            .and_then(|ident| ident.prop.as_ident())
                    })
                })
                .or_else(|| callee_expr.as_ident())
            {
                self.parse_gettext(node, ident);
            }
        }
    }
}
