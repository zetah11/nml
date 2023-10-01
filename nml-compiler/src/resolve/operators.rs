use std::collections::BTreeMap;

use bumpalo::Bump;

use super::{ItemId, Resolver};
use crate::errors::ErrorId;
use crate::names::{Ident, Name};
use crate::source::Span;
use crate::trees::parsed::Affix;
use crate::trees::{parsed, resolved};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub(super) fn apply_expr_run(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        terms: &[parsed::Expr<'_, 'lit>],
    ) -> resolved::Expr<'a, 'lit> {
        let terms = terms
            .iter()
            .map(|expr| self.expr(item_id, gen_scope, expr))
            .collect();

        Precedencer::new(self).unflatten(terms)
    }

    pub(super) fn apply_pattern_run(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        terms: &[parsed::Pattern<'_, 'lit>],
    ) -> resolved::Pattern<'a, 'lit> {
        let terms = terms
            .iter()
            .map(|pattern| self.pattern(item_id, gen_scope, pattern))
            .collect();

        Precedencer::new(self).unflatten(terms)
    }

    fn prefixes<A: Affixable<'a>>(&self, mut fun: A, args: Vec<A>) -> A {
        for arg in args {
            fun = A::apply(self.alloc, fun, arg);
        }

        fun
    }
}

struct Precedencer<'a, 'lit, 'err, 'resolver, A> {
    resolver: &'resolver mut Resolver<'a, 'lit, 'err>,
    infix: Option<(Vec<A>, A, Name)>,
    exprs: Vec<A>,
}

impl<'a, 'lit, 'err, 'resolver, A> Precedencer<'a, 'lit, 'err, 'resolver, A>
where
    A: Affixable<'a>,
{
    pub fn new(resolver: &'resolver mut Resolver<'a, 'lit, 'err>) -> Self {
        Self {
            resolver,
            infix: None,
            exprs: Vec::new(),
        }
    }

    pub fn unflatten(mut self, terms: Vec<A>) -> A {
        for term in terms {
            self.term(term);
        }

        if let Some((mut lhs, op, name)) = self.infix {
            let lhs = {
                let fun = lhs.remove(0);
                self.resolver.prefixes(fun, lhs)
            };

            let rhs = if self.exprs.is_empty() {
                let span = op.span();
                let name = self.resolver.names.get_name(&name);
                let name = self.resolver.names.get_ident(&name.name);
                let error = self.resolver.errors.parse_error(span).infix_function(name);
                A::invalid(error, span)
            } else {
                let fun = self.exprs.remove(0);
                self.resolver.prefixes(fun, self.exprs)
            };

            let fun = A::apply(self.resolver.alloc, op, lhs);
            A::apply(self.resolver.alloc, fun, rhs)
        } else {
            let fun = self.exprs.remove(0);
            self.resolver.prefixes(fun, self.exprs)
        }
    }

    fn term(&mut self, term: A) {
        let (term, name) = term.name();
        if let Some(name) = name {
            match self.resolver.affii.get(&name) {
                Some(&Affix::Postfix) => self.postfix_term(term, name),
                Some(&Affix::Infix) => self.infix_term(term, name),
                _ => self.exprs.push(term),
            }
        } else {
            self.exprs.push(term)
        }
    }

    fn infix_term(&mut self, term: A, name: Name) {
        if let Some((_, op, _)) = self.infix.as_ref() {
            let span = term.span();
            let error = self
                .resolver
                .errors
                .parse_error(span)
                .ambiguous_infix_operators(op.span());
            let expr = A::invalid(error, span);
            self.exprs.push(expr);
        } else if self.exprs.is_empty() {
            let span = term.span();
            let name = self.resolver.names.get_name(&name);
            let name = self.resolver.names.get_ident(&name.name);
            let error = self.resolver.errors.parse_error(span).infix_function(name);
            let expr = A::invalid(error, span);
            self.exprs.push(expr);
        } else {
            let lhs = std::mem::take(&mut self.exprs);
            self.infix = Some((lhs, term, name));
        }
    }

    fn postfix_term(&mut self, term: A, name: Name) {
        if let Some(prev) = self.exprs.pop() {
            let expr = A::apply(self.resolver.alloc, term, prev);
            self.exprs.push(expr);
        } else {
            let span = term.span();
            let name = self.resolver.names.get_name(&name);
            let name = self.resolver.names.get_ident(&name.name);
            let error = self
                .resolver
                .errors
                .parse_error(span)
                .postfix_function(name);
            let expr = A::invalid(error, span);
            self.exprs.push(expr);
        }
    }
}

trait Affixable<'a>: Sized {
    fn invalid(error: ErrorId, span: Span) -> Self;
    fn apply(alloc: &'a Bump, fun: Self, arg: Self) -> Self;

    fn name(self) -> (Self, Option<Name>);

    fn span(&self) -> Span;
}

impl<'a, 'lit> Affixable<'a> for resolved::Expr<'a, 'lit> {
    fn invalid(error: ErrorId, span: Span) -> Self {
        let node = resolved::ExprNode::Invalid(error);
        Self { node, span }
    }

    fn apply(alloc: &'a Bump, fun: Self, arg: Self) -> Self {
        let span = fun.span + arg.span;
        let node = resolved::ExprNode::Apply(alloc.alloc([fun, arg]));
        Self { node, span }
    }

    fn name(self) -> (Self, Option<Name>) {
        if let resolved::ExprNode::Var(name) = &self.node {
            let name = *name;
            (self, Some(name))
        } else {
            (self, None)
        }
    }

    fn span(&self) -> Span {
        self.span
    }
}

impl<'a, 'lit> Affixable<'a> for resolved::Pattern<'a, 'lit> {
    fn invalid(error: ErrorId, span: Span) -> Self {
        let node = resolved::PatternNode::Invalid(error);
        Self { node, span }
    }

    fn apply(alloc: &'a Bump, fun: Self, arg: Self) -> Self {
        let span = fun.span + arg.span;
        let node = resolved::PatternNode::Apply(alloc.alloc([fun, arg]));
        Self { node, span }
    }

    fn name(self) -> (Self, Option<Name>) {
        if let resolved::PatternNode::Constructor(name) = &self.node {
            let name = *name;
            (self, Some(name))
        } else {
            (self, None)
        }
    }

    fn span(&self) -> Span {
        self.span
    }
}
