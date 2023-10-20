use bumpalo::collections::Vec;

use super::Abstractifier;
use crate::errors::ErrorId;
use crate::names::Ident;
use crate::parse::cst;
use crate::trees::parsed as ast;

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
        let span = def.span;
        let pattern = self.type_pattern(def.pattern);
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

            cst::Node::Case(scrutinee, alts) => self.data_body(alts),

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

                ast::Constructor {
                    node: ast::ConstructorNode::Constructor((affix, name), params),
                    span,
                }
            }

            cst::Node::Apply(run) => {
                let [name, params @ ..] = &run[..] else {
                    unreachable!("application runs have at least two terms");
                };

                let Ok(name) = self.data_constructor_name(name) else {
                    todo!()
                };

                let params = self
                    .alloc
                    .alloc_slice_fill_iter(params.iter().map(|thing| self.ty(thing)));

                ast::Constructor {
                    node: ast::ConstructorNode::Constructor(name, params),
                    span,
                }
            }

            _ => {
                todo!()
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
                    _ => todo!(),
                };

                let name = match &name.node {
                    cst::Node::Name(cst::Name::Normal(name)) => name,
                    _ => todo!(),
                };

                let name = self.names.intern(name);

                if let Some(rest) = rest.iter().map(|node| node.span).reduce(|a, b| a + b) {
                    todo!()
                }

                Ok((affix, name))
            }

            _ => todo!(),
        }
    }
}
