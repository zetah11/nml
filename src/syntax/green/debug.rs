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

impl Node {
    /// Concatenate all of the tokens in this tree.
    pub fn write(&self) -> String {
        let mut result = String::with_capacity(self.width);
        let mut stack = vec![self];

        while let Some(node) = stack.pop() {
            match &node.data {
                Data::Node(children) => stack.extend(children.iter().rev()),
                Data::Token(lexeme) => result.push_str(lexeme),
            }
        }

        result
    }

    pub fn check_invariants(&self) {
        assert_eq!(self.width, self.data.width());

        match &self.data {
            Data::Node(children) => {
                for child in children.iter() {
                    child.check_invariants();
                }
            }

            Data::Token(_) => {}
        }
    }
}

impl Data {
    /// Get the width of this node data in bytes.
    pub fn width(&self) -> usize {
        match self {
            Self::Node(children) => children.iter().map(|node| node.width).sum(),
            Self::Token(lexeme) => lexeme.len(),
        }
    }
}
