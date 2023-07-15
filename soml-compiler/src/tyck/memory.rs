use typed_arena::Arena;

use super::tree::Row;
use super::Type;

pub struct Alloc<'a> {
    types: &'a Arena<Type<'a>>,
    records: &'a Arena<Row<'a>>,
}

impl<'a> Alloc<'a> {
    pub fn new(types: &'a Arena<Type<'a>>, records: &'a Arena<Row<'a>>) -> Self {
        Self { types, records }
    }

    pub fn ty(&self, ty: Type<'a>) -> &'a Type<'a> {
        self.types.alloc(ty)
    }

    pub fn row(&self, row: Row<'a>) -> &'a Row<'a> {
        self.records.alloc(row)
    }
}
