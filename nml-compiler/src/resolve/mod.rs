mod dependencies;
mod expr;
mod operators;
mod pattern;
mod types;

use std::collections::BTreeMap;

use bumpalo::Bump;
use log::debug;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Name, Names, ScopeName};
use crate::source::{SourceId, Span};
use crate::topology;
use crate::trees::parsed::Affix;
use crate::trees::{declared, parsed, resolved};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemId(usize);

pub fn resolve<'a, 'b, 'lit>(
    names: &'a Names<'lit>,
    alloc: &'a Bump,
    program: &'b parsed::Source<'b, 'lit>,
) -> resolved::Program<'a, 'lit>
where
    'lit: 'a,
{
    let scratch = Bump::new();
    resolve_program::<'a, '_, 'lit>(names, alloc, &scratch, program)
}

fn resolve_program<'a, 'b: 'b, 'lit>(
    names: &'a Names<'lit>,
    alloc: &'a Bump,
    scratch: &'b Bump,
    program: &'b parsed::Source<'b, 'lit>,
) -> resolved::Program<'a, 'lit>
where
    'lit: 'a,
{
    let mut errors = program.errors.clone();
    let mut resolver = Resolver::new(names, alloc, scratch, &mut errors, program.source);

    let mut items = resolver.items(program.items);
    let graph = items
        .iter()
        .map(|(id, item)| (*id, resolver.dependencies(item)))
        .collect();
    let order = topology::find(&graph);

    let items = alloc.alloc_slice_fill_iter(order.into_iter().map(|component| {
        &*alloc.alloc_slice_fill_iter(
            component
                .into_iter()
                .map(|id| items.remove(id).expect("all item ids are defined")),
        )
    }));

    resolved::Program {
        items,
        defs: resolver.spans,
        errors,
        unattached: program.unattached.clone(),
    }
}

struct Resolver<'a, 'scratch, 'lit, 'err> {
    names: &'a Names<'lit>,
    alloc: &'a Bump,
    scratch: &'scratch Bump,
    errors: &'err mut Errors,

    items: BTreeMap<Name, ItemId>,
    spans: BTreeMap<Name, Span>,
    affii: BTreeMap<Name, Affix>,

    scopes: (Vec<Scope<'lit>>, Scope<'lit>),
    counter: usize,
    item_ids: usize,
}

impl<'a, 'scratch, 'lit, 'err> Resolver<'a, 'scratch, 'lit, 'err> {
    pub fn new(
        names: &'a Names<'lit>,
        alloc: &'a Bump,
        scratch: &'scratch Bump,
        errors: &'err mut Errors,
        source: SourceId,
    ) -> Self {
        let scope = Scope::top_level(source);

        Self {
            names,
            alloc,
            scratch,
            errors,

            items: BTreeMap::new(),
            spans: BTreeMap::new(),
            affii: BTreeMap::new(),

            scopes: (Vec::new(), scope),
            counter: 0,
            item_ids: 0,
        }
    }

    pub fn items(
        &mut self,
        items: &'scratch [parsed::Item<'scratch, 'lit>],
    ) -> BTreeMap<ItemId, resolved::Item<'a, 'lit>> {
        debug!("declaring {} items", items.len());
        let items: Vec<declared::constructored::Item<'scratch, 'lit>> = items
            .iter()
            .map(|item| self.constructor_items(item))
            .collect();

        let items: Vec<declared::Item<'a, 'scratch, 'lit>> = items
            .into_iter()
            .map(|item| self.declare_item(item))
            .collect();

        debug!("resolving {} items", items.len());
        items
            .into_iter()
            .map(|node| (node.id, self.resolve_item(node)))
            .collect()
    }

    fn constructor_items(
        &mut self,
        item: &'scratch parsed::Item<'scratch, 'lit>,
    ) -> declared::constructored::Item<'scratch, 'lit> {
        let id = ItemId(self.item_ids);
        self.item_ids += 1;
        let span = item.span;
        let node = match &item.node {
            parsed::ItemNode::Invalid(e) => declared::constructored::ItemNode::Invalid(*e),
            parsed::ItemNode::Let(pattern, expr, ()) => {
                declared::constructored::ItemNode::Let(pattern, expr, ())
            }

            parsed::ItemNode::Data(pattern, body) => {
                let ctrs = self
                    .scratch
                    .alloc_slice_fill_iter(body.0.iter().map(|ctor| {
                        let parsed::DataConstructor {
                            affix,
                            name,
                            params,
                        } = ctor;
                        let name =
                            self.define_value(id, span, *affix, *name, ValueNamespace::Pattern);
                        declared::constructored::DataConstructor { name, params }
                    }));

                let body = declared::constructored::DataBody(ctrs);

                declared::constructored::ItemNode::Data(pattern, body)
            }
        };

        declared::constructored::Item { node, span, id }
    }

    fn declare_item(
        &mut self,
        item: declared::constructored::Item<'scratch, 'lit>,
    ) -> declared::Item<'a, 'scratch, 'lit> {
        let id = item.id;
        let span = item.span;
        let node = match item.node {
            declared::constructored::ItemNode::Invalid(e) => declared::ItemNode::Invalid(e),
            declared::constructored::ItemNode::Let(pattern, expr, ()) => {
                let mut this_scope = BTreeMap::new();
                let spine = self.function_spine(id, &mut this_scope, pattern);
                let spine: declared::Spine<'scratch, 'lit, resolved::Pattern<'a, 'lit>> =
                    spine.map(|pattern| self.pattern(&mut this_scope, &pattern));

                declared::ItemNode::Let(spine, expr, this_scope)
            }

            declared::constructored::ItemNode::Data(pattern, body) => {
                let pattern = self.type_pattern(id, pattern);
                declared::ItemNode::Data(pattern, body)
            }
        };

        declared::Item { node, span, id }
    }

    fn resolve_item(
        &mut self,
        item: declared::Item<'a, 'scratch, 'lit>,
    ) -> resolved::Item<'a, 'lit> {
        let id = item.id;
        let span = item.span;
        let node = match item.node {
            declared::ItemNode::Invalid(e) => resolved::ItemNode::Invalid(e),
            declared::ItemNode::Let(spine, expr, mut this_scope) => {
                let (pattern, expr) = match spine {
                    declared::Spine::Single(pattern) => {
                        let expr = self.expr(id, &mut this_scope, expr);
                        (pattern, expr)
                    }
                    declared::Spine::Fun { head, args, anno } => {
                        let expr = if let Some(ty) = anno {
                            let span = expr.span + ty.span;
                            let node = parsed::ExprNode::Anno(expr, *ty);
                            self.scratch.alloc(parsed::Expr { node, span })
                        } else {
                            expr
                        };

                        let body = self.lambda(
                            id,
                            &mut this_scope,
                            &args,
                            expr,
                            |this, gen_scope, pattern| this.pattern(gen_scope, pattern),
                        );

                        (head, body)
                    }
                };

                resolved::ItemNode::Let(
                    pattern,
                    expr,
                    self.alloc.alloc_slice_fill_iter(this_scope.into_values()),
                )
            }

            declared::ItemNode::Data(pattern, body) => {
                let mut bad_ctors = Vec::new();
                let ctors = self.alloc.alloc_slice_fill_iter(body.0.iter().map(|ctor| {
                    let declared::constructored::DataConstructor { name, params } = ctor;
                    let mut gen_scope = BTreeMap::new();
                    let params = self.alloc.alloc_slice_fill_iter(
                        params.iter().map(|ty| self.ty(id, &mut gen_scope, ty)),
                    );

                    if !gen_scope.is_empty() {
                        bad_ctors.push(span);
                    }

                    resolved::DataConstructor {
                        name: *name,
                        params,
                    }
                }));

                let body = if bad_ctors.is_empty() {
                    resolved::DataBody(ctors)
                } else {
                    let ctors = self
                        .alloc
                        .alloc_slice_fill_iter(bad_ctors.into_iter().map(|at| {
                            let e = self.errors.name_error(at).implicit_type_var_in_data();
                            let params = self.alloc.alloc([]);
                            resolved::DataConstructor {
                                name: Err(e),
                                params,
                            }
                        }));

                    resolved::DataBody(ctors)
                };

                resolved::ItemNode::Data(pattern, body)
            }
        };

        resolved::Item { id, node, span }
    }

    fn define_type(
        &mut self,
        item: ItemId,
        at: Span,
        affix: Affix,
        ident: Ident<'lit>,
    ) -> Result<Name, ErrorId> {
        let name = self.names.name(self.scopes.1.name, ident);
        let prev = self.items.insert(name, item);
        debug_assert!(prev.is_none());

        if let Some(prev) = self.scopes.1.types.get(&ident) {
            let prev_span = self
                .spans
                .get(prev)
                .expect("all defined names have a defining span");
            let name = self.names.get_ident(&ident);
            Err(self.errors.name_error(at).redefined_type(*prev_span, name))
        } else {
            self.scopes.1.types.insert(ident, name);
            self.spans.insert(name, at);
            self.affii.insert(name, affix);
            Ok(name)
        }
    }

    fn define_value(
        &mut self,
        item: ItemId,
        at: Span,
        affix: Affix,
        ident: Ident<'lit>,
        ns: ValueNamespace,
    ) -> Result<Name, ErrorId> {
        let name = self.names.name(self.scopes.1.name, ident);
        let prev = self.items.insert(name, item);
        debug_assert!(prev.is_none());

        // Note that, if this is a redefinition, we will return an error. To
        // ensure that the redefined name still has an associated definition,
        // check with `get` before actually inserting the new name.
        if let Some((prev, _)) = self.scopes.1.values.get(&ident) {
            let prev_span = self
                .spans
                .get(prev)
                .expect("all defined names have a defining span");
            let name = self.names.get_ident(&ident);
            Err(self.errors.name_error(at).redefined_value(*prev_span, name))
        } else {
            self.scopes.1.values.insert(ident, (name, ns));
            self.spans.insert(name, at);
            self.affii.insert(name, affix);
            Ok(name)
        }
    }

    fn lookup_value(&mut self, name: &Ident) -> Option<(Name, ValueNamespace)> {
        if let Some(name) = self.scopes.1.values.get(name) {
            return Some(*name);
        }

        for scope in self.scopes.0.iter().rev() {
            if let Some(name) = scope.values.get(name) {
                return Some(*name);
            }
        }

        None
    }

    fn scope<F, T>(&mut self, name: Option<Name>, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let mut top = Scope::new(name.map(ScopeName::Item).unwrap_or_else(|| {
            self.counter += 1;
            ScopeName::Anonymous(self.counter)
        }));
        std::mem::swap(&mut self.scopes.1, &mut top);
        self.scopes.0.push(top);

        let result = f(self);

        let top = self
            .scopes
            .0
            .pop()
            .expect("only the `scope` method modifies the scope stack");
        self.scopes.1 = top;

        result
    }
}

#[derive(Clone, Copy, Debug)]
enum ValueNamespace {
    Pattern,
    Value,
}

#[derive(Debug)]
struct Scope<'lit> {
    name: ScopeName,
    values: BTreeMap<Ident<'lit>, (Name, ValueNamespace)>,
    types: BTreeMap<Ident<'lit>, Name>,
}

impl Scope<'_> {
    pub fn new(name: ScopeName) -> Self {
        Self {
            name,
            values: BTreeMap::new(),
            types: BTreeMap::new(),
        }
    }

    pub fn top_level(source: SourceId) -> Self {
        Self {
            name: ScopeName::TopLevel(source),
            values: BTreeMap::new(),
            types: BTreeMap::new(),
        }
    }
}
