use std::collections::BTreeSet;

use crate::names::Name;
use crate::trees::resolved::{Expr, ExprNode, Item, ItemId, ItemNode, Pattern, PatternNode};

use super::Resolver;

impl Resolver<'_, '_> {
    pub fn dependencies(&self, item: &Item) -> BTreeSet<ItemId> {
        match &item.node {
            ItemNode::Invalid(_) => BTreeSet::new(),
            ItemNode::Let(name, body) => {
                let mut ignore = name.iter().copied().collect();
                let mut depends = BTreeSet::new();
                self.in_expr(&mut ignore, &mut depends, body);
                depends
            }
        }
    }

    fn in_expr(&self, ignore: &mut BTreeSet<Name>, out: &mut BTreeSet<ItemId>, expr: &Expr) {
        match &expr.node {
            ExprNode::Invalid(_)
            | ExprNode::Hole
            | ExprNode::Unit
            | ExprNode::Bool(_)
            | ExprNode::Number(_) => {}

            ExprNode::Var(name) if ignore.contains(name) => {}
            ExprNode::Var(name) => {
                out.extend(self.items.get(name).copied());
            }

            ExprNode::If(cond, then, elze) => {
                self.in_expr(ignore, out, cond);
                self.in_expr(ignore, out, then);
                self.in_expr(ignore, out, elze);
            }

            ExprNode::Field(expr, _, _) | ExprNode::Restrict(expr, _) => {
                self.in_expr(ignore, out, expr);
            }

            ExprNode::Record(bindings, extend) => {
                for (_, _, expr) in bindings.iter() {
                    self.in_expr(ignore, out, expr);
                }

                if let Some(expr) = extend {
                    self.in_expr(ignore, out, expr);
                }
            }

            ExprNode::Variant(_) => {}

            ExprNode::Case { scrutinee, cases } => {
                self.in_expr(ignore, out, scrutinee);
                for (pattern, expr) in cases.iter() {
                    self.in_pattern(ignore, out, pattern);
                    self.in_expr(ignore, out, expr);
                }
            }

            ExprNode::Apply(fun, arg) => {
                self.in_expr(ignore, out, fun);
                self.in_expr(ignore, out, arg);
            }

            ExprNode::Lambda(pattern, expr) => {
                self.in_pattern(ignore, out, pattern);
                self.in_expr(ignore, out, expr);
            }

            ExprNode::Let(name, bound, body) => {
                self.in_expr(ignore, out, bound);

                if let Ok(name) = name {
                    ignore.insert(*name);
                }

                self.in_expr(ignore, out, body);
            }
        }
    }

    fn in_pattern(
        &self,
        ignore: &mut BTreeSet<Name>,
        out: &mut BTreeSet<ItemId>,
        pattern: &Pattern,
    ) {
        match &pattern.node {
            PatternNode::Invalid(_) | PatternNode::Wildcard | PatternNode::Unit => {}

            PatternNode::Bind(name) => {
                ignore.insert(*name);
            }

            PatternNode::Named(name) if ignore.contains(name) => {}
            PatternNode::Named(name) => {
                out.extend(self.items.get(name).copied());
            }

            PatternNode::Deconstruct(_, pattern) => {
                self.in_pattern(ignore, out, pattern);
            }

            PatternNode::Apply(fun, arg) => {
                self.in_pattern(ignore, out, fun);
                self.in_pattern(ignore, out, arg);
            }
        }
    }
}
