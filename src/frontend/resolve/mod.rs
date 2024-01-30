mod declare;
mod patterns;
#[allow(clippy::module_inception)]
mod resolve;

mod dependencies;
mod expr;
mod operators;
mod pattern;
mod types;

use std::collections::{BTreeMap, BTreeSet};

use bumpalo::Bump;
use log::debug;

use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::{Ident, Name, Names, ScopeName};
use crate::frontend::source::{SourceId, Span};
use crate::frontend::topology;
use crate::frontend::trees::parsed::Affix;
use crate::frontend::trees::{declared, parsed, resolved};

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
    explicit_universals: BTreeSet<Name>,

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
            explicit_universals: BTreeSet::new(),

            scopes: (Vec::new(), scope),
            counter: 0,
            item_ids: 0,
        }
    }

    pub fn items(
        &mut self,
        items: &'scratch [parsed::Item<'scratch, 'lit>],
    ) -> BTreeMap<ItemId, resolved::Item<'a, 'lit>> {
        let items: Vec<declared::patterns::Item<'scratch, 'lit>> = self.pattern_items(items);
        let items: Vec<declared::Item<'a, 'scratch, 'lit>> = self.declare_items(items);
        self.resolve_items(items)
    }

    fn pattern_items(
        &mut self,
        items: &'scratch [parsed::Item<'scratch, 'lit>],
    ) -> Vec<declared::patterns::Item<'scratch, 'lit>> {
        debug!("patterning {} items", items.len());
        items
            .iter()
            .map(|item| self.constructor_items(item))
            .collect()
    }

    fn declare_items(
        &mut self,
        items: Vec<declared::patterns::Item<'scratch, 'lit>>,
    ) -> Vec<declared::Item<'a, 'scratch, 'lit>> {
        debug!("declaring {} items", items.len());
        items
            .into_iter()
            .map(|item| self.declare_item(item))
            .collect()
    }

    fn resolve_items(
        &mut self,
        items: Vec<declared::Item<'a, 'scratch, 'lit>>,
    ) -> BTreeMap<ItemId, resolved::Item<'a, 'lit>> {
        debug!("resolving {} items", items.len());
        items
            .into_iter()
            .map(|node| (node.id, self.resolve_item(node)))
            .collect()
    }

    fn define_name(
        &mut self,
        item: ItemId,
        at: Span,
        affix: Affix,
        ident: Ident<'lit>,
        kind: Namekind,
        ns: Namespace,
    ) -> Result<Name, ErrorId> {
        match ns {
            Namespace::Type => self.define_type(item, at, affix, ident),
            Namespace::Value => self.define_value(item, at, affix, ident, kind),
        }
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
        kind: Namekind,
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
            self.scopes.1.values.insert(ident, (name, kind));
            self.spans.insert(name, at);
            self.affii.insert(name, affix);
            Ok(name)
        }
    }

    fn lookup_type(&mut self, name: &Ident) -> Option<Name> {
        if let Some(name) = self.scopes.1.types.get(name) {
            return Some(*name);
        }

        for scope in self.scopes.0.iter().rev() {
            if let Some(name) = scope.types.get(name) {
                return Some(*name);
            }
        }

        None
    }

    fn lookup_value(&mut self, name: &Ident) -> Option<(Name, Namekind)> {
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
enum Namespace {
    Type,
    Value,
}

#[derive(Clone, Copy, Debug)]
enum Namekind {
    Pattern,
    Value,
}

#[derive(Debug)]
struct Scope<'lit> {
    name: ScopeName,
    values: BTreeMap<Ident<'lit>, (Name, Namekind)>,
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
