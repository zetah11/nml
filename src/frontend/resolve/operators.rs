use std::collections::BTreeMap;

use bumpalo::Bump;
use log::trace;

use super::{ItemId, Resolver};
use crate::frontend::errors::ErrorId;
use crate::frontend::names::{Ident, Name};
use crate::frontend::source::Span;
use crate::frontend::trees::declared;
use crate::frontend::trees::parsed::Affix;
use crate::frontend::trees::{parsed, resolved};

enum OneOrMany<T> {
    Single(T),
    Many(T, Vec<T>),
}

impl<'a, 'scratch, 'lit> Resolver<'a, 'scratch, 'lit, '_> {
    pub(super) fn apply_expr_run(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        terms: &'scratch [parsed::Expr<'scratch, 'lit>],
    ) -> resolved::Expr<'a, 'lit> {
        trace!("resolving expression run of {} terms", terms.len());

        let terms = terms
            .iter()
            .map(|expr| self.expr(item_id, gen_scope, expr))
            .collect();

        match Precedencer::new(self, self.alloc, item_id).unflatten(terms) {
            OneOrMany::Single(expr) => expr,
            OneOrMany::Many(fun, args) => Precedencer::prefixes(self.alloc, item_id, fun, args),
        }
    }

    pub(super) fn apply_pattern_run(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        terms: &'scratch [parsed::Pattern<'scratch, 'lit>],
    ) -> declared::Spine<'scratch, 'lit, declared::spined::Pattern<'scratch, 'lit>> {
        trace!("resolving pattern run of {} terms", terms.len());

        let terms: Vec<_> = terms
            .iter()
            .map(|pattern| self.single_pattern(item_id, gen_scope, pattern))
            .collect();

        match Precedencer::new(self, self.scratch, item_id).unflatten(terms) {
            OneOrMany::Single(pattern) => declared::Spine::Single(pattern),
            OneOrMany::Many(head, args) => {
                if head.is_constructor() {
                    let pat = Precedencer::prefixes(self.scratch, item_id, head, args);
                    declared::Spine::Single(pat)
                } else {
                    declared::Spine::Fun {
                        head,
                        args,
                        anno: None,
                    }
                }
            }
        }
    }

    pub(super) fn apply_type_run(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        terms: &'scratch [parsed::Type<'scratch, 'lit>],
    ) -> resolved::Type<'a, 'lit> {
        trace!("resolving type run of {} terms", terms.len());

        let terms = terms
            .iter()
            .map(|ty| self.resolve_type(item_id, gen_scope, ty))
            .collect();

        match Precedencer::new(self, self.alloc, item_id).unflatten(terms) {
            OneOrMany::Single(ty) => ty,
            OneOrMany::Many(fun, args) => Precedencer::prefixes(self.alloc, item_id, fun, args),
        }
    }
}

struct Precedencer<'a, 'scratch, 'lit, 'err, 'resolver, 'alloc, A> {
    resolver: &'resolver mut Resolver<'a, 'scratch, 'lit, 'err>,
    alloc: &'alloc Bump,
    item_id: ItemId,
    infix: Option<(Vec<A>, A, Name)>,
    exprs: Vec<A>,
}

impl<'a, 'scratch, 'lit, 'err, 'resolver, 'alloc, A>
    Precedencer<'a, 'scratch, 'lit, 'err, 'resolver, 'alloc, A>
where
    A: Affixable<'alloc>,
{
    pub fn new(
        resolver: &'resolver mut Resolver<'a, 'scratch, 'lit, 'err>,
        alloc: &'alloc Bump,
        item_id: ItemId,
    ) -> Self {
        Self {
            resolver,
            alloc,
            item_id,
            infix: None,
            exprs: Vec::new(),
        }
    }

    pub fn unflatten(mut self, terms: Vec<A>) -> OneOrMany<A> {
        for term in terms {
            self.term(term);
        }

        if let Some((mut lhs, op, name)) = self.infix {
            let lhs = {
                let fun = lhs.remove(0);
                Self::prefixes(self.alloc, self.item_id, fun, lhs)
            };

            let rhs = if self.exprs.is_empty() {
                let span = op.span();
                let name = self.resolver.names.get_name(&name);
                let name = self.resolver.names.get_ident(&name.name);
                let error = self.resolver.errors.parse_error(span).infix_function(name);
                A::invalid(self.item_id, error, span)
            } else {
                let fun = self.exprs.remove(0);
                Self::prefixes(self.alloc, self.item_id, fun, self.exprs)
            };

            let fun = A::apply(self.alloc, self.item_id, op, lhs);
            OneOrMany::Single(A::apply(self.alloc, self.item_id, fun, rhs))
        } else {
            let fun = self.exprs.remove(0);
            OneOrMany::Many(fun, self.exprs)
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
            let expr = A::invalid(self.item_id, error, span);
            self.exprs.push(expr);
        } else if self.exprs.is_empty() {
            let span = term.span();
            let name = self.resolver.names.get_name(&name);
            let name = self.resolver.names.get_ident(&name.name);
            let error = self.resolver.errors.parse_error(span).infix_function(name);
            let expr = A::invalid(self.item_id, error, span);
            self.exprs.push(expr);
        } else {
            let lhs = std::mem::take(&mut self.exprs);
            self.infix = Some((lhs, term, name));
        }
    }

    fn postfix_term(&mut self, term: A, name: Name) {
        if let Some(prev) = self.exprs.pop() {
            let expr = A::apply(self.alloc, self.item_id, term, prev);
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
            let expr = A::invalid(self.item_id, error, span);
            self.exprs.push(expr);
        }
    }

    fn prefixes(alloc: &'alloc Bump, item_id: ItemId, mut fun: A, args: Vec<A>) -> A {
        for arg in args {
            fun = A::apply(alloc, item_id, fun, arg);
        }

        fun
    }
}

trait Affixable<'a>: Sized {
    fn invalid(item_id: ItemId, error: ErrorId, span: Span) -> Self;
    fn apply(alloc: &'a Bump, item_id: ItemId, fun: Self, arg: Self) -> Self;

    fn name(self) -> (Self, Option<Name>);

    fn span(&self) -> Span;
}

impl<'a, 'lit> Affixable<'a> for resolved::Expr<'a, 'lit> {
    fn invalid(_: ItemId, error: ErrorId, span: Span) -> Self {
        let node = resolved::ExprNode::Invalid(error);
        Self { node, span }
    }

    fn apply(alloc: &'a Bump, _: ItemId, fun: Self, arg: Self) -> Self {
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

impl<'a, 'lit> Affixable<'a> for declared::spined::Pattern<'a, 'lit> {
    fn invalid(item_id: ItemId, error: ErrorId, span: Span) -> Self {
        let node = declared::spined::PatternNode::Invalid(error);
        Self {
            node,
            span,
            item_id,
        }
    }

    fn apply(alloc: &'a Bump, item_id: ItemId, fun: Self, arg: Self) -> Self {
        let span = fun.span + arg.span;
        let node = declared::spined::PatternNode::Apply(alloc.alloc([fun, arg]));
        Self {
            node,
            span,
            item_id,
        }
    }

    fn name(self) -> (Self, Option<Name>) {
        if let declared::spined::PatternNode::Constructor(name) = &self.node {
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

impl<'a, 'lit> Affixable<'a> for resolved::Type<'a, 'lit> {
    fn invalid(_: ItemId, error: ErrorId, span: Span) -> Self {
        let node = resolved::TypeNode::Invalid(error);
        Self { node, span }
    }

    fn apply(alloc: &'a Bump, _: ItemId, fun: Self, arg: Self) -> Self {
        let span = fun.span + arg.span;
        let node = resolved::TypeNode::Apply(alloc.alloc([fun, arg]));
        Self { node, span }
    }

    fn name(self) -> (Self, Option<Name>) {
        if let resolved::TypeNode::Named(name) = &self.node {
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
