use std::fmt;

use super::{Data, Node};

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.data {
            Data::Token(lexeme) => fmt::Debug::fmt(&lexeme, f),
            Data::Node(children) => {
                write!(f, "({:?}", self.kind)?;

                for child in children.iter() {
                    write!(f, " {child:?}")?;
                }

                write!(f, ")")
            }
        }
    }
}
