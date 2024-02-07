//! A module type is a collection of identifiers with some arbitrary associated
//! data.  For example the type checker may use the associated data to store
//! type annotations.

pub use identifier::Identifier;

mod identifier;
