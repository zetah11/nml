use std::collections::HashMap;

use lasso::{Key, ThreadedRodeo};

use crate::source::SourceId;

/// A fully qualified name, globally and uniquely identifying a particular
/// entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(usize, SourceId);

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
    Item(Name),
    Anonymous(usize),
}

/// The actual component parts of a fully qualified name, consisting of an
/// optional parent name and an identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qualified {
    pub parent: Option<ScopeName>,
    pub name: Ident,
}

/// A name store is responsible for interning names on a per-source basis.
#[derive(Debug)]
pub struct Names<'a> {
    idents: &'a ThreadedRodeo<Ident>,
    names: HashMap<Name, Qualified>,
    source: SourceId,
    counter: usize,
}

impl<'a> Names<'a> {
    pub fn new(idents: &'a ThreadedRodeo<Ident>, source: SourceId) -> Self {
        Self {
            idents,
            source,
            names: HashMap::new(),
            counter: 0,
        }
    }

    pub fn intern(&self, name: impl AsRef<str>) -> Ident {
        self.idents.get_or_intern(name)
    }

    pub fn label(&self, name: impl AsRef<str>) -> Label {
        Label(self.intern(name))
    }

    pub fn name(&mut self, parent: Option<ScopeName>, name: Ident) -> Name {
        let qualified = Qualified { parent, name };
        self.counter += 1;
        let name = Name(self.counter, self.source);
        self.names.insert(name, qualified);
        name
    }

    pub fn get_ident(&self, ident: &Ident) -> &str {
        self.idents.resolve(ident)
    }

    pub fn get_name(&self, name: &Name) -> &Qualified {
        debug_assert_eq!(name.1, self.source);
        self.names
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
