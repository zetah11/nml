use std::collections::BTreeSet;

use crate::names::Name;
use crate::trees::resolved::{
    Constructor, ConstructorNode, Data, DataNode, Expr, ExprNode, Item, ItemNode, Pattern,
    PatternNode, Type, TypeNode,
};

use super::{ItemId, Resolver};

impl Resolver<'_, '_, '_, '_> {
    pub fn dependencies(&self, item: &Item) -> BTreeSet<ItemId> {
        match &item.node {
            ItemNode::Invalid(_) => BTreeSet::new(),
            ItemNode::Let(pattern, body, _) => {
                let mut ignore = BTreeSet::new();
                let mut depends = BTreeSet::new();
                self.in_pattern(&mut ignore, &mut depends, pattern);
                self.in_expr(&mut ignore, &mut depends, body);
                depends
            }

            ItemNode::Data(pattern, body) => {
                let mut ignore = BTreeSet::new();
                let mut depends = BTreeSet::new();
                self.in_pattern(&mut ignore, &mut depends, pattern);
                self.in_data_body(&mut ignore, &mut depends, body);
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

            ExprNode::If([cond, then, elze]) => {
                self.in_expr(ignore, out, cond);
                self.in_expr(ignore, out, then);
                self.in_expr(ignore, out, elze);
            }

            ExprNode::Anno(expr, ty) => {
                self.in_expr(ignore, out, expr);
                self.in_type(ignore, out, ty);
            }

            ExprNode::Group(expr) => {
                self.in_expr(ignore, out, expr);
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

            ExprNode::Apply([fun, arg]) => {
                self.in_expr(ignore, out, fun);
                self.in_expr(ignore, out, arg);
            }

            ExprNode::Lambda(arrows) => {
                for (pattern, expr) in arrows.iter() {
                    self.in_pattern(ignore, out, pattern);
                    self.in_expr(ignore, out, expr);
                }
            }

            ExprNode::Let(binding, [bound, body], _) => {
                self.in_pattern(ignore, out, binding);
                self.in_expr(ignore, out, bound);
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

            PatternNode::Constructor(name) if ignore.contains(name) => {}
            PatternNode::Constructor(name) => {
                out.extend(self.items.get(name).copied());
            }

            PatternNode::Anno(pattern, ty) => {
                self.in_pattern(ignore, out, pattern);
                self.in_type(ignore, out, ty);
            }

            PatternNode::Group(pattern) => self.in_pattern(ignore, out, pattern),

            PatternNode::Apply([fun, arg]) => {
                self.in_pattern(ignore, out, fun);
                self.in_pattern(ignore, out, arg);
            }
        }
    }

    fn in_data_body(&self, ignore: &mut BTreeSet<Name>, out: &mut BTreeSet<ItemId>, body: &Data) {
        match &body.node {
            DataNode::Invalid(_) => {}
            DataNode::Sum(ctors) => {
                for ctor in *ctors {
                    self.in_constructor(ignore, out, ctor);
                }
            }
        }
    }

    fn in_constructor(
        &self,
        ignore: &mut BTreeSet<Name>,
        out: &mut BTreeSet<ItemId>,
        ctor: &Constructor,
    ) {
        match &ctor.node {
            ConstructorNode::Invalid(_) => {}
            ConstructorNode::Constructor(name, params) => {
                ignore.insert(*name);

                for param in *params {
                    self.in_type(ignore, out, param);
                }
            }
        }
    }

    fn in_type(&self, ignore: &mut BTreeSet<Name>, out: &mut BTreeSet<ItemId>, ty: &Type) {
        let _ = (self, &*ignore, &*out);

        match &ty.node {
            TypeNode::Invalid(_) | TypeNode::Wildcard | TypeNode::Universal(_) => {}
            TypeNode::Function([t, u]) => {
                self.in_type(ignore, out, t);
                self.in_type(ignore, out, u);
            }

            TypeNode::Record(fields) => {
                for (.., ty) in fields.iter() {
                    self.in_type(ignore, out, ty);
                }
            }
        }
    }
}
