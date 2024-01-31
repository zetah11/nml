use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;

use crate::frontend::source::SourceId;

/// A fully qualified name, globally and uniquely identifying a particular
/// entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(usize);

/// A label represents a "detached" name identifying a particular component of a
/// type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Label<'src>(pub Ident<'src>);

/// An identifier directly corresponds to the literal identifiers appearing in
/// the source code.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ident<'src>(&'src str);

/// Globally and uniquely identifies a particular lexical scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScopeName {
    Anonymous(usize),
    Item(Name),
    TopLevel(SourceId),
}

/// The actual component parts of a fully qualified name, consisting of an
/// optional parent name and an identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qualified<'src> {
    pub parent: ScopeName,
    pub name: Ident<'src>,
}

/// A name store is responsible for interning names.
pub struct Names<'src> {
    names: DashMap<Name, Qualified<'src>>,
    counter: AtomicUsize,
}

impl<'src> Names<'src> {
    pub fn new() -> Self {
        Self {
            names: DashMap::new(),
            counter: AtomicUsize::new(0),
        }
    }

    pub fn intern(&self, name: &'src str) -> Ident<'src> {
        Ident(name)
    }

    pub fn label(&self, name: &'src str) -> Label<'src> {
        Label(self.intern(name))
    }

    pub fn name(&self, parent: ScopeName, name: Ident<'src>) -> Name {
        let qualified = Qualified { parent, name };
        let name = Name(self.counter.fetch_add(1, Ordering::SeqCst));
        self.names.insert(name, qualified);
        name
    }

    pub fn get_ident<'b>(&self, ident: &Ident<'b>) -> &'b str {
        ident.0
    }

    pub fn get_name(&self, name: &Name) -> Qualified<'src> {
        *self
            .names
            .get(name)
            .expect("names from separate name stores are never mixed")
    }
}
