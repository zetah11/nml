use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;
use lasso::{Key, ThreadedRodeo};

use crate::source::SourceId;

/// A fully qualified name, globally and uniquely identifying a particular
/// entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(usize);

/// A label represents a "detached" name identifying a particular component of a
/// type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Label(pub Ident);

/// An identifier directly corresponds to the literal identifiers appearing in
/// the source code.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ident(usize);

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
pub struct Qualified {
    pub parent: ScopeName,
    pub name: Ident,
}

/// A name store is responsible for interning names.
#[derive(Debug)]
pub struct Names<'a> {
    idents: &'a ThreadedRodeo<Ident>,
    names: DashMap<Name, Qualified>,
    counter: AtomicUsize,
}

impl<'a> Names<'a> {
    pub fn new(idents: &'a ThreadedRodeo<Ident>) -> Self {
        Self {
            idents,
            names: DashMap::new(),
            counter: AtomicUsize::new(0),
        }
    }

    pub fn intern(&self, name: impl AsRef<str>) -> Ident {
        self.idents.get_or_intern(name)
    }

    pub fn label(&self, name: impl AsRef<str>) -> Label {
        Label(self.intern(name))
    }

    pub fn name(&self, parent: ScopeName, name: Ident) -> Name {
        let qualified = Qualified { parent, name };
        let name = Name(self.counter.fetch_add(1, Ordering::SeqCst));
        self.names.insert(name, qualified);
        name
    }

    pub fn get_ident(&self, ident: &Ident) -> &str {
        self.idents.resolve(ident)
    }

    pub fn get_name(&self, name: &Name) -> Qualified {
        *self
            .names
            .get(name)
            .expect("names from separate name stores are never mixed")
    }
}

// SAFETY: `Ident` is a dumb newtype over usizes, so `try_from_usize` and
// `into_usize` are exactly symmetrical.
unsafe impl Key for Ident {
    fn into_usize(self) -> usize {
        self.0
    }

    fn try_from_usize(value: usize) -> Option<Self> {
        Some(Self(value))
    }
}
