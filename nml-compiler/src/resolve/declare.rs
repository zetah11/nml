use std::collections::BTreeMap;

use bumpalo::Bump;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Name, Names, ScopeName};
use crate::source::{SourceId, Span};
use crate::trees::{declared, parsed};

use super::ItemId;

pub fn declare<'a>(
    alloc: &'a Bump,
    names: &'a Names<'a>,
    tree: &parsed::Source,
) -> declared::Source<'a> {
    let mut declarer = Declarer::new(alloc, names, tree.source, tree.errors.clone());

    let items = tree.items.iter().map(|item| declarer.declare_item(item));
    let items = alloc.alloc_slice_fill_iter(items);

    declared::Source {
        items,
        errors: declarer.errors,
        unattached: tree.unattached.clone(),
        source: tree.source,
        names: declarer.names,
        defines: declarer.spans,
    }
}

struct Declarer<'a> {
    alloc: &'a Bump,
    store: &'a Names<'a>,
    errors: Errors,
    names: BTreeMap<Ident, Name>,
    spans: BTreeMap<Name, (Span, ItemId)>,

    scope: ScopeName,
    count: usize,
}

impl<'a> Declarer<'a> {
    pub fn new(alloc: &'a Bump, store: &'a Names<'a>, source: SourceId, errors: Errors) -> Self {
        Self {
            alloc,
            store,
            errors,
            names: BTreeMap::new(),
            spans: BTreeMap::new(),

            scope: ScopeName::TopLevel(source),
            count: 0,
        }
    }

    pub fn declare_item(&mut self, item: &parsed::Item) -> declared::Item<'a> {
        let id = ItemId(self.count);
        self.count += 1;

        let span = item.span;
        let node = match &item.node {
            parsed::ItemNode::Invalid(e) => declared::ItemNode::Invalid(*e),
            parsed::ItemNode::Let(name, span, body) => {
                let name = name.and_then(|ident| self.declare(id, ident, *span));
                let body = self.declare_expr(body);
                declared::ItemNode::Let(name, (), body)
            }
        };

        declared::Item { id, node, span }
    }

    fn declare_expr(&mut self, expr: &parsed::Expr) -> declared::Expr<'a> {
        let span = expr.span;
        let node = match &expr.node {
            parsed::ExprNode::Invalid(e) => declared::ExprNode::Invalid(*e),

            parsed::ExprNode::Hole => declared::ExprNode::Hole,
            parsed::ExprNode::Unit => declared::ExprNode::Unit,

            parsed::ExprNode::Small(name) => declared::ExprNode::Small(*name),
            parsed::ExprNode::Big(name) => declared::ExprNode::Big(*name),

            parsed::ExprNode::Bool(v) => declared::ExprNode::Bool(*v),
            parsed::ExprNode::Number(num) => declared::ExprNode::Number(num.clone()),

            parsed::ExprNode::If(cond, then, elze) => {
                let cond = self.declare_expr(cond);
                let cond = self.alloc.alloc(cond);
                let then = self.declare_expr(then);
                let then = self.alloc.alloc(then);
                let elze = self.declare_expr(elze);
                let elze = self.alloc.alloc(elze);
                declared::ExprNode::If(cond, then, elze)
            }

            parsed::ExprNode::Field(of, label, label_span) => {
                let of = self.declare_expr(of);
                let of = self.alloc.alloc(of);
                declared::ExprNode::Field(of, *label, *label_span)
            }

            parsed::ExprNode::Record(fields, extend) => {
                let fields = self.alloc.alloc_slice_fill_with(fields.len(), |idx| {
                    let (label, label_span, expr) = &fields[idx];
                    let expr = self.declare_expr(expr);
                    (*label, *label_span, expr)
                });

                let extend =
                    extend.as_ref().map(|expr| &*self.alloc.alloc(self.declare_expr(expr)));
                declared::ExprNode::Record(fields, extend)
            }

            parsed::ExprNode::Restrict(of, label) => {
                let of = self.declare_expr(of);
                let of = self.alloc.alloc(of);
                declared::ExprNode::Restrict(of, *label)
            }

            parsed::ExprNode::Case { scrutinee, cases } => {
                let scrutinee = self.declare_expr(scrutinee);
                let scrutinee = self.alloc.alloc(scrutinee);

                let cases = self.alloc.alloc_slice_fill_with(cases.len(), |idx| {
                    let (pattern, expr) = &cases[idx];
                    let pattern = self.declare_pattern(pattern);
                    let expr = self.declare_expr(expr);
                    (pattern, expr)
                });

                declared::ExprNode::Case { scrutinee, cases }
            }

            parsed::ExprNode::Apply(fun, arg) => {
                let fun = self.declare_expr(fun);
                let fun = self.alloc.alloc(fun);
                let arg = self.declare_expr(arg);
                let arg = self.alloc.alloc(arg);
                declared::ExprNode::Apply(fun, arg)
            }

            parsed::ExprNode::Lambda(pattern, expr) => {
                let pattern = self.declare_pattern(pattern);
                let expr = self.declare_expr(expr);
                let expr = self.alloc.alloc(expr);
                declared::ExprNode::Lambda(pattern, expr)
            }

            parsed::ExprNode::Let(name, name_span, bound, body) => {
                let bound = self.declare_expr(bound);
                let bound = self.alloc.alloc(bound);
                let body = self.declare_expr(body);
                let body = self.alloc.alloc(body);
                declared::ExprNode::Let(*name, *name_span, bound, body)
            }

            parsed::ExprNode::Var(v) => match *v {},
            parsed::ExprNode::Variant(v) => match *v {},
        };

        declared::Expr { node, span }
    }

    fn declare_pattern(&mut self, pattern: &parsed::Pattern) -> declared::Pattern<'a> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => declared::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => declared::PatternNode::Wildcard,
            parsed::PatternNode::Unit => declared::PatternNode::Unit,
            parsed::PatternNode::Small(name) => declared::PatternNode::Small(*name),
            parsed::PatternNode::Big(name) => declared::PatternNode::Big(*name),
            parsed::PatternNode::Apply(fun, arg) => {
                let fun = self.declare_pattern(fun);
                let fun = self.alloc.alloc(fun);
                let arg = self.declare_pattern(arg);
                let arg = self.alloc.alloc(arg);
                declared::PatternNode::Apply(fun, arg)
            }

            parsed::PatternNode::Bind(v) | parsed::PatternNode::Named(v) => match *v {},
            parsed::PatternNode::Deconstruct(v, _) => match *v {},
        };

        declared::Pattern { node, span }
    }

    fn declare(&mut self, id: ItemId, ident: Ident, span: Span) -> Result<Name, ErrorId> {
        let name = self.store.name(self.scope, ident);
        self.names.insert(ident, name);

        if let Some((prev, _)) = self.spans.insert(name, (span, id)) {
            let name = self.store.get_ident(&ident);
            Err(self.errors.name_error(span).redefined_value(prev, name))
        } else {
            Ok(name)
        }
    }
}
