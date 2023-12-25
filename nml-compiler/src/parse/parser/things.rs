use log::trace;

use crate::parse::cst::{LetKw, Name, Node, Thing, ValueDef};
use crate::parse::tokens::Token;
use crate::source::Span;

use super::Parser;

impl<'a, 'err, I: Iterator<Item = (Result<Token<'a>, ()>, Span)>> Parser<'a, 'err, I> {
    /// Parse the current token stream with the assumption of being a finite
    /// program.
    pub fn top_level(&mut self) -> Vec<&'a Thing<'a>> {
        let mut things = Vec::new();

        while self.next.is_some() {
            let mut err = None;

            while self.peek(Self::THING_STARTS).is_none() && self.next.is_some() {
                trace!("skipping token");
                err.get_or_insert(self.current_span);
                self.advance();
            }

            if let Some(span) = err {
                let e = self.errors.parse_error(span).unexpected_token();
                let node = Node::Invalid(e);
                things.push(&*self.alloc.alloc(Thing { node, span }));
            }

            if self.next.is_some() {
                things.push(self.thing());
            }

            for (e, span) in self.parse_errors.drain(..).rev() {
                let node = Node::Invalid(e);
                things.push(self.alloc.alloc(Thing { node, span }));
            }
        }

        things
    }

    const THING_STARTS: &'static [Token<'static>] = &[
        Token::Let,
        Token::Data,
        Token::Case,
        Token::Name(""),
        Token::Symbol(""),
        Token::Universal(""),
        Token::Number(""),
        Token::Underscore,
        Token::Ellipses,
        Token::Infix,
        Token::Postfix,
        Token::LeftParen,
        Token::LeftBrace,
        Token::Pipe,
    ];

    /// ```abnf
    /// thing = item{lambda}
    /// ```
    pub fn thing(&mut self) -> &'a Thing<'a> {
        self.item(Self::lambda)
    }

    /// ```abnf
    /// simple = item{apply}
    /// ```
    pub(super) fn simple(&mut self) -> &'a Thing<'a> {
        self.item(Self::anno)
    }

    /// ```abnf
    /// item{default} = let / if / case / default
    /// ```
    fn item(&mut self, default: impl FnOnce(&mut Self) -> &'a Thing<'a>) -> &'a Thing<'a> {
        if let Some(opener) = self.consume(Token::Let) {
            self.let_def(LetKw::Let, opener)
        } else if let Some(opener) = self.consume(Token::Data) {
            self.let_def(LetKw::Data, opener)
        } else if let Some(opener) = self.consume(Token::Case) {
            self.case(opener)
        } else {
            default(self)
        }
    }

    /// ```abnf
    /// let = "let" def *("and" def) ["in" thing]
    /// ```
    fn let_def(&mut self, kw: LetKw, opener: Span) -> &'a Thing<'a> {
        trace!("parse let-def");

        let primary = self.def(Some(opener));
        let mut others = Vec::new();

        while let Some(and_kw) = self.consume(Token::And) {
            others.push(self.def(Some(and_kw)));
        }

        let within = self.consume(Token::In).map(|_| self.thing());

        trace!("done let");

        let span = opener + self.closest_span();
        let node = Node::Let {
            kw: (kw, opener),
            defs: (primary, others),
            within,
        };

        self.alloc.alloc(Thing { node, span })
    }

    const DEF_STARTS: &'static [Token<'static>] = &[
        Token::Name(""),
        Token::Symbol(""),
        Token::Universal(""),
        Token::Number(""),
        Token::Underscore,
        Token::Ellipses,
        Token::Infix,
        Token::Postfix,
        Token::LeftParen,
        Token::LeftBrace,
    ];

    /// ```abnf
    /// def = apply ["=" thing]
    /// ```
    fn def(&mut self, opener: Option<Span>) -> ValueDef<'a> {
        trace!("parse def");

        let pattern = self.anno();
        let definition = self.consume(Token::Equal).map(|_| self.thing());

        trace!("done def");

        let span = opener.unwrap_or(pattern.span) + self.closest_span();
        ValueDef {
            span,
            pattern,
            definition,
        }
    }

    /// ```abnf
    /// case = "case" [arrow] [lambda] "end"
    /// ```
    fn case(&mut self, opener: Span) -> &'a Thing<'a> {
        trace!("parse `case`");

        let scrutinee = self.peek(Self::ANNO_STARTS).map(|_| self.simple());
        let thing = if self.peek(Self::LAMBDA_STARTS).is_some() {
            self.lambda()
        } else {
            let span = self.closest_span();
            let node = Node::Alt(Vec::new());
            self.alloc.alloc(Thing { node, span })
        };

        let end = self.consume(Token::End).unwrap_or_else(|| {
            let e = self
                .errors
                .parse_error(opener)
                .missing_end("case", self.current_span);
            let span = self.closest_span();
            self.parse_errors.push((e, span));
            span
        });

        trace!("done case");

        let span = opener + end;
        let node = Node::Case(scrutinee, thing);
        self.alloc.alloc(Thing { node, span })
    }

    const LAMBDA_STARTS: &'static [Token<'static>] = &[
        Token::Name(""),
        Token::Symbol(""),
        Token::Universal(""),
        Token::Number(""),
        Token::Underscore,
        Token::Ellipses,
        Token::Infix,
        Token::Postfix,
        Token::LeftParen,
        Token::LeftBrace,
        Token::Pipe,
    ];

    const ANNO_STARTS: &'static [Token<'static>] = &[
        Token::Name(""),
        Token::Symbol(""),
        Token::Universal(""),
        Token::Number(""),
        Token::Underscore,
        Token::Ellipses,
        Token::Infix,
        Token::Postfix,
        Token::LeftParen,
        Token::LeftBrace,
    ];

    /// ```abnf
    /// anno = apply [":" apply]
    /// ```
    fn anno(&mut self) -> &'a Thing<'a> {
        let expr = self.apply();

        if self.consume(Token::Colon).is_some() {
            let anno = self.apply();
            let span = expr.span + anno.span;
            let node = Node::Anno(expr, anno);
            self.alloc.alloc(Thing { node, span })
        } else {
            expr
        }
    }

    /// ```abnf
    /// apply = 1*field
    /// ```
    fn apply(&mut self) -> &'a Thing<'a> {
        trace!("parsing apply");

        let expr = self.field();
        let mut span = expr.span;
        let mut args = vec![expr];

        while self.peek(Self::FIELD_STARTS).is_some() {
            let arg = self.field();
            span += arg.span;
            args.push(arg);
        }

        trace!("done apply");

        if args.len() > 1 {
            let node = Node::Apply(args);
            self.alloc.alloc(Thing { node, span })
        } else {
            expr
        }
    }

    const FIELD_STARTS: &'static [Token<'static>] = &[
        Token::Name(""),
        Token::Symbol(""),
        Token::Universal(""),
        Token::Number(""),
        Token::Underscore,
        Token::Ellipses,
        Token::Infix,
        Token::Postfix,
        Token::LeftParen,
        Token::LeftBrace,
    ];

    /// ```abnf
    /// field = base *("." name)
    /// ```
    fn field(&mut self) -> &'a Thing<'a> {
        trace!("parse field");

        let thing = self.base();
        let mut fields = Vec::new();

        while self.consume(Token::Dot).is_some() {
            if let Some(name) = self.name() {
                fields.push(name);
            } else {
                let span = self.closest_span();
                let e = self.errors.parse_error(span).expected_name();
                let node = Node::Invalid(e);
                return self.alloc.alloc(Thing { node, span });
            }
        }

        trace!("done field");

        if let Some((_, end)) = fields.last() {
            let span = thing.span + *end;
            let node = Node::Field(thing, fields);
            self.alloc.alloc(Thing { node, span })
        } else {
            thing
        }
    }

    /// ```abnf
    /// base  = name / NUMBER / "_" / "..." / "infix" / "postfix"
    /// base =/ "(" thing ")"
    /// base =/ "{" *(def ",") [def] ["|" thing] "}"
    /// ```
    fn base(&mut self) -> &'a Thing<'a> {
        let (node, span) = if let Some((name, span)) = self.name() {
            trace!("name");
            let node = Node::Name(name);
            (node, span)
        } else if let Some((number, span)) = self.number() {
            trace!("number");
            let node = Node::Number(number);
            (node, span)
        } else if let Some(span) = self.consume(Token::Infix) {
            trace!("infix");
            let node = Node::Infix;
            (node, span)
        } else if let Some(span) = self.consume(Token::Postfix) {
            trace!("postfix");
            let node = Node::Postfix;
            (node, span)
        } else if let Some(span) = self.consume(Token::Underscore) {
            trace!("wildcard");
            let node = Node::Wildcard;
            (node, span)
        } else if let Some(span) = self.consume(Token::Ellipses) {
            trace!("ellipses");
            let node = Node::Ellipses;
            (node, span)
        } else if let Some(opener) = self.consume(Token::LeftParen) {
            trace!("parse group");
            let thing = self.thing();
            trace!("done group");
            if let Some(closer) = self.consume(Token::RightParen) {
                let span = opener + closer;
                let node = Node::Group(thing);
                (node, span)
            } else {
                let e = self
                    .errors
                    .parse_error(opener)
                    .unclosed_paren(self.current_span);
                let span = self.closest_span();
                let node = Node::Invalid(e);
                (node, span)
            }
        } else if let Some(opener) = self.consume(Token::LeftBrace) {
            self.record(opener)
        } else {
            let span = self.current_span;
            let e = self.errors.parse_error(span).unexpected_token();
            let node = Node::Invalid(e);
            (node, span)
        };

        self.alloc.alloc(Thing { node, span })
    }

    fn record(&mut self, opener: Span) -> (Node<'a>, Span) {
        trace!("parse record");
        let mut defs = Vec::new();
        let mut expected_comma = None;

        loop {
            if self.peek(Self::DEF_STARTS).is_none() {
                break;
            }

            if let Some(span) = expected_comma.take() {
                let e = self.errors.parse_error(span).expected_comma();
                self.parse_errors.push((e, span));
            }

            if self.peek(Self::DEF_STARTS).is_some() {
                defs.push(self.def(None));
            }

            expected_comma = self
                .consume(Token::Comma)
                .is_none()
                .then_some(self.current_span);
        }

        trace!("done record");

        if let Some(end) = self.consume(Token::RightBrace) {
            let span = opener + end;
            let node = Node::Record { defs };
            (node, span)
        } else {
            let e = self
                .errors
                .parse_error(opener)
                .unclosed_brace(self.current_span);
            let span = self.closest_span();
            let node = Node::Invalid(e);
            (node, span)
        }
    }

    /// ```abnf
    /// name = NAME / OPERATOR / UNIVERSAL
    /// ```
    fn name(&mut self) -> Option<(Name<'a>, Span)> {
        let (name, span) = match self.next.as_ref()? {
            (Token::Name(name), span) => (Name::Normal(name), *span),
            (Token::Symbol(name), span) => (Name::Normal(name), *span),
            (Token::Universal(name), span) => (Name::Universal(name), *span),
            _ => return None,
        };

        self.advance();
        Some((name, span))
    }

    fn number(&mut self) -> Option<(&'a str, Span)> {
        let (num, span) = match self.next.as_ref()? {
            (Token::Number(num), span) => (*num, *span),
            _ => return None,
        };

        self.advance();
        Some((num, span))
    }
}
