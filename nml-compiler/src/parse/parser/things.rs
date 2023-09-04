use log::trace;

use crate::parse::cst::{Name, Node, Thing, ValueDef, ValueDefKw};
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

    const THING_STARTS: &[Token<'static>] = &[
        Token::Let,
        Token::If,
        Token::Case,
        Token::SmallName(""),
        Token::BigName(""),
        Token::Operator(""),
        Token::Number(""),
        Token::Underscore,
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
    fn simple(&mut self) -> &'a Thing<'a> {
        self.item(Self::apply)
    }

    /// ```abnf
    /// item{default} = let / if / case / default
    /// ```
    fn item(&mut self, default: impl FnOnce(&mut Self) -> &'a Thing<'a>) -> &'a Thing<'a> {
        if let Some(opener) = self.consume(Token::Let) {
            self.let_fun(ValueDefKw::Let, opener)
        } else if let Some(opener) = self.consume(Token::Fun) {
            self.let_fun(ValueDefKw::Fun, opener)
        } else if let Some(opener) = self.consume(Token::If) {
            self.if_do(opener)
        } else if let Some(opener) = self.consume(Token::Case) {
            self.case(opener)
        } else {
            default(self)
        }
    }

    /// ```abnf
    /// let = ("let" / "fun") def *("and" def) ["in" thing]
    /// ```
    fn let_fun(&mut self, kw: ValueDefKw, opener: Span) -> &'a Thing<'a> {
        trace!("parse let/fun");

        let primary = self.def(Some(opener));
        let mut others = Vec::new();

        while let Some(and_kw) = self.consume(Token::And) {
            others.push(self.def(Some(and_kw)));
        }

        let within = self.consume(Token::In).map(|_| self.thing());

        trace!("done let/fun");

        let span = opener + self.closest_span();
        let node = Node::Let {
            keyword: (kw, opener),
            defs: (primary, others),
            within,
        };
        self.alloc.alloc(Thing { node, span })
    }

    const DEF_STARTS: &[Token<'static>] = &[
        Token::SmallName(""),
        Token::BigName(""),
        Token::Operator(""),
        Token::Number(""),
        Token::Underscore,
        Token::LeftParen,
        Token::LeftBrace,
    ];

    /// ```abnf
    /// def = apply "=" thing
    /// ```
    fn def(&mut self, opener: Option<Span>) -> ValueDef<'a> {
        trace!("parse def");

        let pattern = self.apply();
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
    /// if = "if" thing "do" thing ("else" thing / "end")
    /// ```
    fn if_do(&mut self, opener: Span) -> &'a Thing<'a> {
        trace!("parse `if`");

        let conditional = self.thing();
        let Some(_do_kw) = self.consume(Token::Do) else {
            let span = self.current_span;
            let e = self.errors.parse_error(opener).missing_do("if", span);
            let node = Node::Invalid(e);
            return self.alloc.alloc(Thing { node, span });
        };

        let consequence = self.thing();

        let thing = if let Some(_else_kw) = self.consume(Token::Else) {
            let alternative = self.thing();
            let span = opener + alternative.span;
            let node = Node::If {
                conditional,
                consequence,
                alternative: Some(alternative),
            };
            self.alloc.alloc(Thing { node, span })
        } else {
            let end = self.consume(Token::End).unwrap_or_else(|| {
                let e = self
                    .errors
                    .parse_error(opener)
                    .missing_end("if", self.current_span);
                let span = self.closest_span();
                self.parse_errors.push((e, span));
                span
            });

            let span = opener + end;
            let node = Node::If {
                conditional,
                consequence,
                alternative: None,
            };
            self.alloc.alloc(Thing { node, span })
        };

        trace!("done `if`");
        thing
    }

    /// ```abnf
    /// case = "case" [arrow] [lambda] "end"
    /// ```
    fn case(&mut self, opener: Span) -> &'a Thing<'a> {
        trace!("parse `case`");

        let scrutinee = self.peek(Self::ARROW_STARTS).map(|_| self.arrow());
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

    const LAMBDA_STARTS: &[Token<'static>] = &[
        Token::SmallName(""),
        Token::BigName(""),
        Token::Operator(""),
        Token::Number(""),
        Token::Underscore,
        Token::LeftParen,
        Token::LeftBrace,
        Token::Pipe,
    ];

    /// ```abnf
    /// lambda = ["|"] arrow *("|" arrow)
    /// ```
    fn lambda(&mut self) -> &'a Thing<'a> {
        let opener = self.consume(Token::Pipe);
        let expr = self.arrow();
        let mut span = opener.unwrap_or(expr.span);
        let mut alts = vec![expr];

        while self.consume(Token::Pipe).is_some() {
            let expr = self.arrow();
            span += expr.span;
            alts.push(expr);
        }

        if alts.len() == 1 {
            alts.remove(0)
        } else {
            let node = Node::Alt(alts);
            &*self.alloc.alloc(Thing { node, span })
        }
    }

    const ARROW_STARTS: &[Token<'static>] = &[
        Token::SmallName(""),
        Token::BigName(""),
        Token::Operator(""),
        Token::Number(""),
        Token::Underscore,
        Token::LeftParen,
        Token::LeftBrace,
    ];

    /// ```abnf
    /// arrow = simple ["=>" arrow]
    /// ```
    fn arrow(&mut self) -> &'a Thing<'a> {
        let expr = self.simple();
        if let Some(arrow) = self.consume(Token::EqualArrow) {
            let result = self.arrow();
            let span = expr.span + arrow + result.span;
            let node = Node::Arrow(expr, result);
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

    const FIELD_STARTS: &[Token<'static>] = &[
        Token::SmallName(""),
        Token::BigName(""),
        Token::Operator(""),
        Token::Number(""),
        Token::Underscore,
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
    /// base  = name / NUMBER / "_"
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
        } else if let Some(opener) = self.consume(Token::LeftParen) {
            trace!("parse group");
            let thing = self.thing();
            trace!("done group");
            if self.consume(Token::RightParen).is_none() {
                let e = self
                    .errors
                    .parse_error(opener)
                    .unclosed_paren(self.current_span);
                let span = self.closest_span();
                let node = Node::Invalid(e);
                return self.alloc.alloc(Thing { node, span });
            } else {
                return thing;
            }
        } else if let Some(opener) = self.consume(Token::LeftBrace) {
            trace!("parse record");
            let mut defs = Vec::new();
            let mut extends = Vec::new();

            let mut expected_comma = None;

            loop {
                if self.peek(Self::DEF_STARTS).is_none() && self.peek(Token::Pipe).is_none() {
                    break;
                }

                if self.peek(Self::DEF_STARTS).is_some() {
                    if let Some(span) = expected_comma.take() {
                        let e = self.errors.parse_error(span).expected_comma();
                        self.parse_errors.push((e, span));
                    }

                    defs.push(self.def(None));
                }

                if self.consume(Token::Pipe).is_some() {
                    extends.push(self.thing());
                }

                expected_comma = self
                    .consume(Token::Comma)
                    .is_none()
                    .then_some(self.current_span);
            }

            trace!("done record");

            if let Some(end) = self.consume(Token::RightBrace) {
                let span = opener + end;
                let node = Node::Record { defs, extends };
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
        } else {
            let span = self.current_span;
            let e = self.errors.parse_error(span).unexpected_token();
            let node = Node::Invalid(e);
            (node, span)
        };

        self.alloc.alloc(Thing { node, span })
    }

    /// ```abnf
    /// name = SMALL / BIG / OPERATOR
    /// ```
    fn name(&mut self) -> Option<(Name<'a>, Span)> {
        let (name, span) = match self.next.as_ref()? {
            (Token::SmallName(name), span) => (Name::Small(name), *span),
            (Token::BigName(name), span) => (Name::Big(name), *span),
            (Token::Operator(name), span) => (Name::Operator(name), *span),
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
