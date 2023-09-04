use internment::Arena;
use malachite::Integer;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Literal {
    Integer(Integer),
}

impl Literal {
    pub fn int(arena: &Arena<Literal>, value: Integer) -> &Integer {
        let Literal::Integer(ref int) = arena.intern(Literal::Integer(value)).into_ref();
        int
    }
}
