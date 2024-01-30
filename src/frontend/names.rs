use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;
use internment::Arena;

use crate::frontend::source::SourceId;

/// A fully qualified name, globally and uniquely identifying a particular
/// entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(usize);

/// A label represents a "detached" name identifying a particular component of a
/// type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Label<'name>(pub Ident<'name>);

/// An identifier directly corresponds to the literal identifiers appearing in
/// the source code.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ident<'name>(&'name str);

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
pub struct Qualified<'name> {
    pub parent: ScopeName,
    pub name: Ident<'name>,
}

/// A name store is responsible for interning names.
pub struct Names<'name> {
    intern: &'name Arena<str>,
    names: DashMap<Name, Qualified<'name>>,
    counter: AtomicUsize,
    builtins: Builtins<'name>,
}

impl<'name> Names<'name> {
    pub fn new(intern: &'name Arena<str>) -> Self {
        let minus_arrow = Ident(intern.intern("->").into_ref());
        let builtins = Builtins { minus_arrow };

        Self {
            intern,
            names: DashMap::new(),
            counter: AtomicUsize::new(0),
            builtins,
        }
    }

    pub fn builtins(&self) -> &Builtins<'name> {
        &self.builtins
    }

    pub fn intern(&self, name: impl AsRef<str>) -> Ident<'name> {
        Ident(self.intern.intern(name.as_ref()).into_ref())
    }

    pub fn label(&self, name: impl AsRef<str>) -> Label<'name> {
        Label(self.intern(name))
    }

    pub fn name(&self, parent: ScopeName, name: Ident<'name>) -> Name {
        let qualified = Qualified { parent, name };
        let name = Name(self.counter.fetch_add(1, Ordering::SeqCst));
        self.names.insert(name, qualified);
        name
    }

    pub fn get_ident<'b>(&self, ident: &Ident<'b>) -> &'b str {
        ident.0
    }

    pub fn get_name(&self, name: &Name) -> Qualified<'name> {
        *self
            .names
            .get(name)
            .expect("names from separate name stores are never mixed")
    }
}

pub struct Builtins<'name> {
    minus_arrow: Ident<'name>,
}

impl<'name> Builtins<'name> {
    /// `->`
    pub fn minus_arrow(&self) -> Ident<'name> {
        self.minus_arrow
    }
}
