use typed_arena::Arena;

use super::tree::RecordRow;
use super::Type;

pub struct Alloc<'a> {
    types: &'a Arena<Type<'a>>,
    records: &'a Arena<RecordRow<'a>>,
}

impl<'a> Alloc<'a> {
    pub fn new(types: &'a Arena<Type<'a>>, records: &'a Arena<RecordRow<'a>>) -> Self {
        Self { types, records }
    }

    pub fn ty(&self, ty: Type<'a>) -> &'a Type<'a> {
        self.types.alloc(ty)
    }

    pub fn record(&self, row: RecordRow<'a>) -> &'a RecordRow<'a> {
        self.records.alloc(row)
    }
}
