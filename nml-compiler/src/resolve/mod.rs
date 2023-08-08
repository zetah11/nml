mod dependencies;
mod expr;
mod pattern;

use std::collections::BTreeMap;

use bumpalo::Bump;
use log::debug;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Name, Names, ScopeName};
use crate::source::{SourceId, Span};
use crate::topology;
use crate::trees::{declared, parsed, resolved};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemId(usize);

pub fn resolve<'a>(
    names: &'a Names<'a>,
    alloc: &'a Bump,
    program: &parsed::Source<'a>,
) -> resolved::Program<'a> {
    let mut errors = program.errors.clone();
    let mut resolver = Resolver::new(names, alloc, &mut errors, program.source);

    let mut items = resolver.items(program.items);
    let graph = items.iter().map(|(id, item)| (*id, resolver.dependencies(item))).collect();
    let order = topology::find(&graph);

    let items = alloc.alloc_slice_fill_iter(order.into_iter().map(|component| {
        &*alloc.alloc_slice_fill_iter(
            component.into_iter().map(|id| items.remove(id).expect("all item ids are defined")),
        )
    }));

    resolved::Program {
        items,
        defs: resolver.spans,
        errors,
        unattached: program.unattached.clone(),
    }
}

struct Resolver<'a, 'err> {
    names: &'a Names<'a>,
    alloc: &'a Bump,
    errors: &'err mut Errors,

    items: BTreeMap<Name, ItemId>,
    spans: BTreeMap<Name, Span>,

    scopes: (Vec<Scope>, Scope),
    counter: usize,
    item_ids: usize,
}

impl<'a, 'err> Resolver<'a, 'err> {
    pub fn new(
        names: &'a Names<'a>,
        alloc: &'a Bump,
        errors: &'err mut Errors,
        source: SourceId,
    ) -> Self {
        let scope = Scope::top_level(source);

        Self {
            names,
            alloc,
            errors,

            items: BTreeMap::new(),
            spans: BTreeMap::new(),

            scopes: (Vec::new(), scope),
            counter: 0,
            item_ids: 0,
        }
    }

    pub fn items(&mut self, items: &'a [parsed::Item<'a>]) -> BTreeMap<ItemId, resolved::Item<'a>> {
        debug!("declaring {} items", items.len());
        let items: Vec<_> = items.iter().map(|item| self.declare_item(item)).collect();

        debug!("resolving {} items", items.len());
        items.into_iter().map(|node| (node.id, self.resolve_item(node))).collect()
    }

    fn declare_item(&mut self, item: &'a parsed::Item<'a>) -> declared::Item<'a> {
        let id = ItemId(self.item_ids);
        self.item_ids += 1;
        let span = item.span;
        let node = match &item.node {
            parsed::ItemNode::Invalid(e) => declared::ItemNode::Invalid(*e),
            parsed::ItemNode::Let(pattern, expr) => {
                let pattern = self.pattern(id, pattern);
                declared::ItemNode::Let(pattern, expr)
            }
        };

        declared::Item { node, span, id }
    }

    fn resolve_item(&mut self, item: declared::Item<'a>) -> resolved::Item<'a> {
        let id = item.id;
        let span = item.span;
        let node = match item.node {
            declared::ItemNode::Invalid(e) => resolved::ItemNode::Invalid(e),
            declared::ItemNode::Let(pattern, expr) => {
                let expr = self.expr(id, expr);
                resolved::ItemNode::Let(pattern, expr)
            }
        };

        resolved::Item { id, node, span }
    }

    fn define_value(&mut self, item: ItemId, at: Span, ident: Ident) -> Result<Name, ErrorId> {
        let name = self.names.name(self.scopes.1.name, ident);

        let prev = self.items.insert(name, item);
        debug_assert!(prev.is_none());

        let result = if let Some(prev) = self.scopes.1.values.insert(ident, name) {
            let prev_span = self.spans.get(&prev).expect("all defined names have a defining span");
            let name = self.names.get_ident(&ident);
            Err(self.errors.name_error(at).redefined_value(*prev_span, name))
        } else {
            Ok(name)
        };

        self.spans.insert(name, at);
        result
    }

    fn lookup_value(&mut self, name: &Ident) -> Option<Name> {
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

        let top = self.scopes.0.pop().expect("only the `scope` method modifies the scope stack");
        self.scopes.1 = top;

        result
    }
}

#[derive(Debug)]
struct Scope {
    name: ScopeName,
    values: BTreeMap<Ident, Name>,
}

impl Scope {
    pub fn new(name: ScopeName) -> Self {
        Self { name, values: BTreeMap::new() }
    }

    pub fn top_level(source: SourceId) -> Self {
        Self { name: ScopeName::TopLevel(source), values: BTreeMap::new() }
    }
}
