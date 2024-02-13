#![expect(unused)]

use crate::syntax::green::{Data, Kind, Node};

pub fn parse(tokens: impl IntoIterator<Item = Node>) -> Node {
    let tokens = tokens.into_iter();
    let mut parser = Parser {
        tokens,
        current: None,
        next: None,
        trailing: Vec::new(),
        stack: Vec::new(),
    };

    SOURCE.parse(&mut parser);

    debug_assert!(parser.current.is_none());
    debug_assert!(parser.next.is_none());
    debug_assert!(parser.trailing.is_empty());
    debug_assert_eq!(1, parser.stack.len());

    parser.stack.pop().unwrap()
}

struct Parser<I> {
    tokens: I,
    current: Option<Node>,
    next: Option<Node>,

    /// Skippables between `current` and `next`
    trailing: Vec<Node>,
    stack: Vec<Node>,
}

/// Parser bases
impl<I> Parser<I>
where
    I: Iterator<Item = Node>,
{
    fn advance(&mut self) {
        self.stack.extend(self.current.take());
        self.stack.append(&mut self.trailing);
        self.current = self.next.take();

        loop {
            let Some(node) = self.tokens.next() else {
                break;
            };

            if !node.kind.is_skipped() {
                self.next = Some(node);
                break;
            }

            self.trailing.push(node);
        }
    }

    fn is_done(&self) -> bool {
        self.current.is_none()
    }

    fn peek(&self, kind: Kind) -> bool {
        self.peek_any(&[kind])
    }

    fn peek_any(&self, kinds: &[Kind]) -> bool {
        self.current
            .as_ref()
            .is_some_and(|node| kinds.contains(&node.kind))
    }

    fn consume(&mut self, kind: Kind) -> bool {
        self.consume_any(&[kind])
    }

    fn consume_any(&mut self, kinds: &[Kind]) -> bool {
        if self.peek_any(kinds) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: Kind) {
        self.expect_any(&[kind])
    }

    fn expect_any(&mut self, kinds: &[Kind]) {
        if !self.consume_any(kinds) {
            // TODO: deal with errors somehow
        }
    }

    fn collect(&mut self, kind: Kind, body: impl FnOnce(&mut Self)) {
        let start_idx = self.stack.len();
        body(self);
        let end_idx = self.stack.len();

        debug_assert!(end_idx >= start_idx, "{end_idx} < {start_idx}");

        // Only wrap the nodes in a new node if there is more than one item on
        // the stack
        if 1 < end_idx - start_idx {
            let children: Vec<_> = self.stack.drain(start_idx..end_idx).collect();
            let width = children.iter().map(|node| node.width).sum();

            self.stack.push(Node {
                width,
                kind,
                data: Data::Node(children.into()),
            });
        }
    }

    fn always_collect(&mut self, kind: Kind, body: impl FnOnce(&mut Self)) {
        let start_idx = self.stack.len();
        body(self);
        let end_idx = self.stack.len();

        debug_assert!(end_idx >= start_idx, "{end_idx} < {start_idx}");

        let children: Vec<_> = self.stack.drain(start_idx..end_idx).collect();
        let width = children.iter().map(|node| node.width).sum();

        self.stack.push(Node {
            width,
            kind,
            data: Data::Node(children.into()),
        });
    }
}

trait Production {
    const FIRST: &'static [Kind];
    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>);
}

/// Concatenate multiple slices at compile time.  The first argument is an
/// initializer, used only to create the array.
///
/// ```
/// const A: &'static [char] = &['a', 'b', 'c'];
/// const B: &'static [char] = &['1', '2', '3'];
/// const C: &'static [char] = constcat!('-'; A, B);
/// assert_eq!(&['a', 'b', 'c', '1', '2', '3'], C);
/// ```
macro_rules! constcat {
    ($init:expr; $($slice:expr),*) => {
        &{
            const LEN: usize = $( $slice.len() + )* 0;
            let mut arr = [$init; LEN];
            let mut base = 0;

            $({
                let mut i = 0;
                while i < $slice.len() {
                    arr[base + i] = $slice[i];
                    i += 1;
                }
                base += $slice.len();
            })*

            if base != LEN {
                panic!("bad invocation");
            }

            arr
        }
    };
}

/// ```abnf
/// source = thing *thing
/// ```
const SOURCE: Source = Source;

struct Source;

impl Production for Source {
    const FIRST: &'static [Kind] = Thing::FIRST;

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.always_collect(Kind::Source, |parser| {
            // Skip initial whitespace
            parser.advance();

            // Put the first non-empty token into `current`
            parser.advance();

            loop {
                if parser.peek_any(Thing::FIRST) {
                    THING.parse(parser);
                } else if parser.is_done() {
                    break;
                } else {
                    parser.advance();
                }
            }
        })
    }
}

/// ```abnf
/// thing = kw-or{disjoined}
/// kw-or{default} = case / scoped / default
/// ```
const THING: Thing = Thing;
struct Thing;

impl Production for Thing {
    const FIRST: &'static [Kind] = constcat!(Kind::Ampersand;
        Case::FIRST,
        Scoped::FIRST,
        Disjoined::FIRST
    );

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        if parser.peek_any(Case::FIRST) {
            CASE.parse(parser)
        } else if parser.peek_any(Scoped::FIRST) {
            SCOPED.parse(parser)
        } else if parser.peek_any(Disjoined::FIRST) {
            DISJOINED.parse(parser)
        } else {
            // TODO: better error recovery here
            parser.expect_any(Self::FIRST);
        }
    }
}

/// ```abnf
/// simple = kw-or{conjoined}
/// ````
const SIMPLE: Simple = Simple;
struct Simple;

impl Production for Simple {
    const FIRST: &'static [Kind] = constcat!(Kind::Ampersand;
        Case::FIRST,
        Scoped::FIRST,
        Conjoined::FIRST
    );

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        if parser.peek_any(Case::FIRST) {
            CASE.parse(parser)
        } else if parser.peek_any(Scoped::FIRST) {
            SCOPED.parse(parser)
        } else if parser.peek_any(Conjoined::FIRST) {
            CONJOINED.parse(parser)
        } else {
            // TODO: better error recovery here
            parser.expect_any(Self::FIRST);
        }
    }
}

/// ```abnf
/// case = "case" [implied] [disjoined] "end"
/// ```
const CASE: Case = Case;
struct Case;

impl Production for Case {
    const FIRST: &'static [Kind] = &[Kind::Case];

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(Kind::CaseGroup, |parser| {
            parser.expect(Kind::Case);

            if parser.peek_any(Implied::FIRST) {
                IMPLIED.parse(parser);
            }

            if parser.peek_any(Disjoined::FIRST) {
                DISJOINED.parse(parser);
            }

            parser.expect(Kind::End);
        })
    }
}

/// ```abnf
/// scoped = def-group ["in" thing]
/// ```
const SCOPED: Scoped = pair(Kind::Scoped, DEF_GROUP, Kind::In, THING);

type Scoped = Pair<DefGroup, Thing>;

/// ```abnf
/// def-group = ("data" / "let") def *("and" def)
/// ```
const DEF_GROUP: DefGroup = DefGroup;
struct DefGroup;

impl Production for DefGroup {
    const FIRST: &'static [Kind] = &[Kind::Data, Kind::Let];

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(Kind::DefinitionGroup, |parser| {
            parser.expect_any(Self::FIRST);

            loop {
                DEF.parse(parser);

                if !parser.consume(Kind::And) {
                    break;
                }

                if !parser.peek_any(Def::FIRST) {
                    break;
                }
            }
        })
    }
}

/// ```abnf
/// def = apply ["=" thing]
/// ```
const DEF: Def = pair(Kind::Definition, APPLY, Kind::Equal, THING);
type Def = Pair<Apply, Thing>;

/// ```abnf
/// disjoined = ["|"] implied *("|" implied)
/// ```
const DISJOINED: Disjoined = Disjoined;
struct Disjoined;

impl Production for Disjoined {
    const FIRST: &'static [Kind] = constcat!(Kind::Pipe; Implied::FIRST, &[Kind::Pipe]);

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(Kind::Disjoined, |parser| {
            parser.consume(Kind::Pipe);

            loop {
                IMPLIED.parse(parser);

                if !parser.consume(Kind::Pipe) {
                    break;
                }

                if !parser.peek_any(Implied::FIRST) {
                    break;
                }
            }
        })
    }
}

/// ```abnf
/// implied = conjoined *("=>" conjoined)
/// ```
const IMPLIED: Implied = separated(Kind::Implied, Kind::EqualArrow, CONJOINED);
type Implied = Separated<Conjoined>;

/// ```abnf
/// conjoined = apply *("&" apply)
/// ```
const CONJOINED: Conjoined = separated(Kind::Conjoined, Kind::Ampersand, ANNO);
type Conjoined = Separated<Anno>;

/// ```abnf
/// anno = apply [":" apply]
/// ```
const ANNO: Anno = pair(Kind::Annotate, APPLY, Kind::Colon, APPLY);
type Anno = Pair<Apply, Apply>;

/// ```abnf
/// apply = qual *qual
/// ```
const APPLY: Apply = repeated(Kind::Apply, QUAL);
type Apply = Repeated<Qual>;

/// ```abnf
/// qual = atom *("." atom)
/// ```
const QUAL: Qual = separated(Kind::Qualified, Kind::Dot, ATOM);
type Qual = Separated<Atom>;

/// ```abnf
/// atom  = NAME / PRE-TICK / POST-TICK
/// atom =/ NUMBER / "_" / "..." / "infix" / "postfix"
/// atom =/ paren-group / brace-group
/// ````
const ATOM: Atom = Atom;
struct Atom;

impl Production for Atom {
    const FIRST: &'static [Kind] = &[
        Kind::Name,
        Kind::PreTick,
        Kind::PostTick,
        Kind::Number,
        Kind::Underscore,
        Kind::Ellipses,
        Kind::Infix,
        Kind::Postfix,
        Kind::LeftParen,
        Kind::LeftBrace,
    ];

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        if parser.peek_any(ParenGroup::FIRST) {
            PAREN_GROUP.parse(parser)
        } else if parser.peek_any(BraceGroup::FIRST) {
            BRACE_GROUP.parse(parser)
        } else {
            parser.expect_any(Self::FIRST)
        }
    }
}

/// ```abnf
/// paren-group = "(" thing ")"
/// ```
const PAREN_GROUP: ParenGroup = ParenGroup;
struct ParenGroup;

impl Production for ParenGroup {
    const FIRST: &'static [Kind] = &[Kind::LeftParen];

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(Kind::ParenGroup, |parser| {
            parser.expect(Kind::LeftParen);
            THING.parse(parser);
            parser.expect(Kind::RightParen);
        })
    }
}

/// ```abnf
/// brace-group = "{" *(def ",") [def] "}"
/// ```
const BRACE_GROUP: BraceGroup = BraceGroup;
struct BraceGroup;

impl Production for BraceGroup {
    const FIRST: &'static [Kind] = &[Kind::LeftBrace];

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(Kind::BraceGroup, |parser| {
            parser.expect(Kind::LeftBrace);

            loop {
                if parser.peek_any(Def::FIRST) {
                    DEF.parse(parser);
                }

                if !parser.consume(Kind::Comma) {
                    break;
                }
            }

            parser.expect(Kind::RightBrace);
        })
    }
}

/// ```abnf
/// pair{first, between, second} = first [between second]
/// ```
const fn pair<A, B>(wrapping: Kind, first: A, between: Kind, second: B) -> Pair<A, B> {
    Pair {
        wrapping,
        between,
        first,
        second,
    }
}

struct Pair<A, B> {
    wrapping: Kind,
    between: Kind,
    first: A,
    second: B,
}

impl<A: Production, B: Production> Production for Pair<A, B> {
    const FIRST: &'static [Kind] = A::FIRST;

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(self.wrapping, |parser| {
            self.first.parse(parser);

            if parser.consume(self.between) {
                self.second.parse(parser);
            }
        })
    }
}

/// ```abnf
/// repeated{inner} = inner+
/// ```
const fn repeated<P>(wrapping: Kind, production: P) -> Repeated<P> {
    Repeated {
        wrapping,
        inner: production,
    }
}

struct Repeated<P> {
    wrapping: Kind,
    inner: P,
}

impl<P: Production> Production for Repeated<P> {
    const FIRST: &'static [Kind] = P::FIRST;

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(self.wrapping, |parser| loop {
            self.inner.parse(parser);

            if !parser.peek_any(P::FIRST) {
                break;
            }
        })
    }
}

/// ```abnf
/// separated{by, inner} = inner *(by inner)
/// ```
const fn separated<P>(wrapping: Kind, by: Kind, production: P) -> Separated<P> {
    Separated {
        wrapping,
        by,
        inner: production,
    }
}

struct Separated<P> {
    wrapping: Kind,
    by: Kind,
    inner: P,
}

impl<P: Production> Production for Separated<P> {
    const FIRST: &'static [Kind] = P::FIRST;

    fn parse<I: Iterator<Item = Node>>(&self, parser: &mut Parser<I>) {
        parser.collect(self.wrapping, |parser| loop {
            self.inner.parse(parser);

            if !parser.consume(self.by) {
                break;
            }

            if !parser.peek_any(P::FIRST) {
                break;
            }
        })
    }
}
