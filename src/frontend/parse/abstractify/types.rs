use super::Abstractifier;
use crate::frontend::errors::ErrorId;
use crate::frontend::names::Label;
use crate::frontend::parse::cst::{self, ValueDef};
use crate::frontend::source::Span;
use crate::frontend::trees::parsed as ast;

impl<'a, 'src> Abstractifier<'a, 'src, '_> {
    pub fn ty(&mut self, node: &cst::Thing<'_, 'src>) -> ast::Type<'a, 'src> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::TypeNode::Invalid(*e),
            cst::Node::Wildcard => ast::TypeNode::Wildcard,

            cst::Node::Name(cst::Name::Normal(name)) => {
                let name = self.names.intern(name);
                ast::TypeNode::Named(name)
            }

            cst::Node::Name(cst::Name::Universal(name)) => {
                let name = self.names.intern(name);
                ast::TypeNode::Universal(name)
            }

            cst::Node::Group(node) => {
                return self.ty(node);
            }

            cst::Node::Record { defs } => {
                let fields: Vec<_> = defs.iter().map(|def| self.field(def)).collect();
                let fields = self.alloc.alloc_slice_fill_iter(fields);
                ast::TypeNode::Record(fields)
            }

            cst::Node::Apply(types) => {
                let types = self
                    .alloc
                    .alloc_slice_fill_iter(types.iter().map(|ty| self.ty(ty)));
                ast::TypeNode::Apply(types)
            }

            _ => {
                let e = self.errors.parse_error(span).expected_type();
                ast::TypeNode::Invalid(e)
            }
        };

        ast::Type { node, span }
    }

    fn field(
        &mut self,
        def: &ValueDef<'_, 'src>,
    ) -> (Result<Label<'src>, ErrorId>, Span, ast::Type<'a, 'src>) {
        let (name, ty) = self.anno(def.pattern);
        let (name, name_span) = match name {
            (Ok(name), _) => self.normal_name(name),
            (Err(e), name_span) => (Err(e), name_span),
        };

        let name = name.map(Label);

        let mut ty = match ty {
            (Ok(ty), _) => self.ty(ty),
            (Err(e), span) => {
                let node = ast::TypeNode::Invalid(e);
                ast::Type { node, span }
            }
        };

        if let Some(body) = def.definition {
            let span = body.span;
            let e = self.errors.parse_error(span).record_type_field_definition();
            let node = ast::TypeNode::Invalid(e);
            ty = ast::Type { node, span };
        }

        (name, name_span, ty)
    }

    fn anno<'b, 's>(&mut self, node: &'b cst::Thing<'b, 's>) -> (Bit<'b, 's>, Bit<'b, 's>) {
        let span = node.span;
        match &node.node {
            cst::Node::Invalid(e) => ((Err(*e), span), (Err(*e), span)),
            cst::Node::Group(node) => self.anno(node),
            cst::Node::Anno(a, b) => ((Ok(a), a.span), (Ok(b), b.span)),

            cst::Node::Name(cst::Name::Normal(name) | cst::Name::Universal(name)) => {
                let e = self.errors.parse_error(span).expected_annotation(name);
                ((Ok(node), span), (Err(e), span))
            }

            _ => {
                let e = self.errors.parse_error(span).expected_annotated_name();
                ((Err(e), span), (Err(e), span))
            }
        }
    }
}

type Bit<'a, 's> = (Result<&'a cst::Thing<'a, 's>, ErrorId>, Span);
