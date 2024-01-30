use bumpalo::collections::Vec;

use super::Abstractifier;
use crate::frontend::errors::ErrorId;
use crate::frontend::names::Ident;
use crate::frontend::parse::cst;
use crate::frontend::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn item(&mut self, into: &mut Vec<ast::Item<'a, 'lit>>, node: &cst::Thing) {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::ItemNode::Invalid(*e),
            cst::Node::Let {
                kw: (cst::LetKw::Let, _),
                defs,
                within,
            } => {
                if let Some(within) = within {
                    let e = self
                        .errors
                        .parse_error(within.span)
                        .item_definition_with_body();
                    self.parse_errors.push((e, within.span));
                }

                into.reserve_exact(defs.1.len() + 1);
                into.push(self.single_value(&defs.0));
                into.extend(defs.1.iter().map(|def| self.single_value(def)));
                return;
            }

            cst::Node::Let {
                kw: (cst::LetKw::Data, _),
                defs,
                within,
            } => {
                if let Some(within) = within {
                    let e = self
                        .errors
                        .parse_error(within.span)
                        .item_definition_with_body();
                    self.parse_errors.push((e, within.span));
                }

                into.reserve_exact(defs.1.len() + 1);
                into.push(self.single_data_type(&defs.0));
                into.extend(defs.1.iter().map(|def| self.single_value(def)));
                return;
            }

            cst::Node::Group(item) => {
                return self.item(into, item);
            }

            _ => {
                let e = self.errors.parse_error(span).expected_item();
                ast::ItemNode::Invalid(e)
            }
        };

        into.push(ast::Item { node, span });
    }

    fn single_value(&mut self, def: &cst::ValueDef) -> ast::Item<'a, 'lit> {
        let pattern = self.pattern(def.pattern);

        let body = def
            .definition
            .map(|node| self.expr(node))
            .unwrap_or_else(|| {
                let span = pattern.span;
                let e = self.errors.parse_error(span).missing_definition();
                let node = ast::ExprNode::Invalid(e);
                ast::Expr { node, span }
            });

        let span = def.span;
        let node = ast::ItemNode::Let(pattern, body, ());
        ast::Item { node, span }
    }

    fn single_data_type(&mut self, def: &cst::ValueDef) -> ast::Item<'a, 'lit> {
        let pattern = self.pattern(def.pattern);

        let span = def.span;
        let body = if let Some(body) = def.definition {
            self.data_body(body)
        } else {
            ast::Data {
                node: ast::DataNode::Sum(self.alloc.alloc([])),
                span,
            }
        };

        let node = ast::ItemNode::Data(pattern, body);
        ast::Item { node, span }
    }

    fn data_body(&mut self, node: &cst::Thing) -> ast::Data<'a, 'lit> {
        let span = node.span;
        match &node.node {
            cst::Node::Group(thing) => self.data_body(thing),

            cst::Node::Alt(things) => {
                let ctors = self
                    .alloc
                    .alloc_slice_fill_iter(things.iter().map(|thing| self.data_constructor(thing)));
                ast::Data {
                    node: ast::DataNode::Sum(ctors),
                    span,
                }
            }

            cst::Node::Case(scrutinee, alts) => {
                if let Some(scrutinee) = scrutinee {
                    let span = scrutinee.span;
                    let e = self.errors.parse_error(span).scrutinee_in_sum_data_type();
                    ast::Data {
                        node: ast::DataNode::Invalid(e),
                        span,
                    }
                } else {
                    self.data_body(alts)
                }
            }

            _ => {
                let ctor = self.data_constructor(node);
                let ctors = self.alloc.alloc([ctor]);
                ast::Data {
                    node: ast::DataNode::Sum(ctors),
                    span,
                }
            }
        }
    }

    fn data_constructor(&mut self, node: &cst::Thing) -> ast::Constructor<'a, 'lit> {
        let span = node.span;
        match &node.node {
            cst::Node::Group(thing) => self.data_constructor(thing),

            cst::Node::Name(cst::Name::Normal(name)) => {
                let affix = ast::Affix::Prefix;
                let name = self.names.intern(name);
                let params = self.alloc.alloc([]);

                let node = ast::ConstructorNode::Constructor((affix, name), params);
                ast::Constructor { node, span }
            }

            cst::Node::Apply(run) => {
                let [name, params @ ..] = &run[..] else {
                    unreachable!("application runs have at least two terms");
                };

                let name = match self.data_constructor_name(name) {
                    Ok(name) => name,
                    Err(e) => {
                        let node = ast::ConstructorNode::Invalid(e);
                        return ast::Constructor { node, span };
                    }
                };

                let params = self
                    .alloc
                    .alloc_slice_fill_iter(params.iter().map(|thing| self.ty(thing)));

                let node = ast::ConstructorNode::Constructor(name, params);
                ast::Constructor { node, span }
            }

            _ => {
                let e = self.errors.parse_error(span).expected_constructor_name();
                let node = ast::ConstructorNode::Invalid(e);
                ast::Constructor { node, span }
            }
        }
    }

    fn data_constructor_name(
        &mut self,
        node: &cst::Thing,
    ) -> Result<(ast::Affix, Ident<'lit>), ErrorId> {
        match &node.node {
            cst::Node::Invalid(e) => Err(*e),
            cst::Node::Group(thing) => self.data_constructor_name(thing),

            cst::Node::Name(cst::Name::Normal(name)) => {
                let affix = ast::Affix::Prefix;
                let name = self.names.intern(name);
                Ok((affix, name))
            }

            cst::Node::Apply(run) => {
                let [affix, name, rest @ ..] = &run[..] else {
                    unreachable!("application runs have at least two terms");
                };

                let affix = match &affix.node {
                    cst::Node::Infix => ast::Affix::Infix,
                    cst::Node::Postfix => ast::Affix::Postfix,

                    cst::Node::Name(_) => {
                        let span = rest
                            .iter()
                            .map(|node| node.span)
                            .fold(name.span, |a, b| a + b);
                        return Err(self
                            .errors
                            .parse_error(span)
                            .constructor_parameters_not_after_name());
                    }

                    _ => {
                        return Err(self
                            .errors
                            .parse_error(affix.span)
                            .expected_constructor_name())
                    }
                };

                let name = match &name.node {
                    cst::Node::Name(cst::Name::Normal(name)) => name,
                    _ => {
                        return Err(self
                            .errors
                            .parse_error(name.span)
                            .expected_constructor_name())
                    }
                };

                let name = self.names.intern(name);

                if let Some(rest) = rest.iter().map(|node| node.span).reduce(|a, b| a + b) {
                    Err(self
                        .errors
                        .parse_error(rest)
                        .constructor_parameters_not_after_name())
                } else {
                    Ok((affix, name))
                }
            }

            _ => Err(self
                .errors
                .parse_error(node.span)
                .expected_constructor_name()),
        }
    }
}
