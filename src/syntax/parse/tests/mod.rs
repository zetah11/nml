use smol_str::SmolStr;

use super::parse;
use crate::syntax::green::{Data, Kind, Node};

mod impls;
mod invariants;
mod junk;
mod lexing;
mod lossless;

#[test]
fn single_item() {
    let line = r"let name = 56 + 1234";
    let expected = Node::source([Node::definition_group([
        Node::token_let(),
        Node::token_space(" "),
        Node::definition([
            Node::token_name("name"),
            Node::token_space(" "),
            Node::token_equal(),
            Node::token_space(" "),
            Node::apply([
                Node::token_number("56"),
                Node::token_space(" "),
                Node::token_name("+"),
                Node::token_space(" "),
                Node::token_number("1234"),
            ]),
        ]),
    ])]);

    let actual = parse(line);
    assert_eq!(expected, actual);
}

#[test]
fn lambda_to_let() {
    let line = r"x => let y";
    let expected = Node::source([Node::implied([
        Node::token_name("x"),
        Node::token_space(" "),
        Node::token_equal_arrow(),
        Node::token_space(" "),
        Node::definition_group([
            Node::token_let(),
            Node::token_space(" "),
            Node::definition([Node::token_name("y")]),
        ]),
    ])]);

    let actual = parse(line);
    assert_eq!(expected, actual);
}

#[test]
fn multiple_lines() {
    let lines = "data a  \n let b = 5";
    let expected = Node::source([
        Node::definition_group([
            Node::token_data(),
            Node::token_space(" "),
            Node::definition([
                Node::token_name("a"),
                Node::token_space("  \n"),
                Node::token_space(" "),
            ]),
        ]),
        Node::definition_group([
            Node::token_let(),
            Node::token_space(" "),
            Node::definition([
                Node::token_name("b"),
                Node::token_space(" "),
                Node::token_equal(),
                Node::token_space(" "),
                Node::token_number("5"),
            ]),
        ]),
    ]);

    let actual = parse(lines);
    assert_eq!(expected, actual);
}

/// `A | B => x | y => z` should parse as `((A | B) => x) | (y => z)`.  The
/// last pipe `|` differentiates cases while the first pipe differentiates
/// branches.  Only before an arrow does the pipe bind tighter.
#[test]
fn or_patterns() {
    let line = "A|B=>x|y=>z";
    let expected = Node::source([Node::disjoined([
        Node::implied([
            Node::disjoined([
                Node::token_name("A"),
                Node::token_pipe(),
                Node::token_name("B"),
            ]),
            Node::token_equal_arrow(),
            Node::token_name("x"),
        ]),
        Node::token_pipe(),
        Node::implied([
            Node::token_name("y"),
            Node::token_equal_arrow(),
            Node::token_name("z"),
        ]),
    ])]);

    let actual = parse(line);
    assert_eq!(expected, actual);
}

impl Node {
    fn source(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::Source, children)
    }

    fn definition_group(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::DefinitionGroup, children)
    }

    fn definition(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::Definition, children)
    }

    fn disjoined(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::Disjoined, children)
    }

    fn implied(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::Implied, children)
    }

    fn apply(children: impl IntoIterator<Item = Self>) -> Self {
        Self::make_node(Kind::Apply, children)
    }

    fn token_data() -> Self {
        Self::make_token(Kind::Data, "data")
    }

    fn token_let() -> Self {
        Self::make_token(Kind::Let, "let")
    }

    fn token_equal() -> Self {
        Self::make_token(Kind::Equal, "=")
    }

    fn token_equal_arrow() -> Self {
        Self::make_token(Kind::EqualArrow, "=>")
    }

    fn token_pipe() -> Self {
        Self::make_token(Kind::Pipe, "|")
    }

    fn token_name(name: &str) -> Self {
        Self::make_token(Kind::Name, name)
    }

    fn token_number(number: &str) -> Self {
        Self::make_token(Kind::Number, number)
    }

    fn token_space(ws: &str) -> Self {
        Self::make_token(Kind::Whitespace, ws)
    }

    fn make_node(kind: Kind, children: impl IntoIterator<Item = Self>) -> Self {
        let children: Vec<_> = children.into_iter().collect();
        let width = children.iter().map(|node| node.width).sum();

        Self {
            kind,
            width,
            data: Data::Node(children.into()),
        }
    }

    fn make_token(kind: Kind, lexeme: impl Into<SmolStr>) -> Self {
        let lexeme: SmolStr = lexeme.into();
        let width = lexeme.len();

        Self {
            kind,
            width,
            data: Data::Token(lexeme),
        }
    }
}
