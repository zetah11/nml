use super::{ItemId, Namekind, Resolver};
use crate::frontend::trees::{declared, parsed};

impl<'a, 'scratch, 'lit, 'err> Resolver<'a, 'scratch, 'lit, 'err> {
    pub(super) fn constructor_items(
        &mut self,
        item: &'scratch parsed::Item<'scratch, 'lit>,
    ) -> declared::patterns::Item<'scratch, 'lit> {
        let id = ItemId(self.item_ids);
        self.item_ids += 1;
        let span = item.span;
        let node = match &item.node {
            parsed::ItemNode::Invalid(e) => declared::patterns::ItemNode::Invalid(*e),
            parsed::ItemNode::Let(pattern, expr, ()) => {
                declared::patterns::ItemNode::Let(pattern, expr, ())
            }

            parsed::ItemNode::Data(pattern, body) => {
                let body = self.constructor_data(id, body);
                declared::patterns::ItemNode::Data(pattern, body)
            }
        };

        declared::patterns::Item { node, span, id }
    }

    fn constructor_data(
        &mut self,
        id: ItemId,
        data: &'scratch parsed::Data<'scratch, 'lit>,
    ) -> declared::patterns::Data<'scratch, 'lit> {
        let span = data.span;
        let node = match &data.node {
            parsed::DataNode::Invalid(e) => declared::patterns::DataNode::Invalid(*e),
            parsed::DataNode::Sum(ctors) => {
                let ctors = self.scratch.alloc_slice_fill_iter(
                    ctors
                        .iter()
                        .map(|ctor| self.constructor_constructor(id, ctor)),
                );

                declared::patterns::DataNode::Sum(ctors)
            }
        };

        declared::patterns::Data { node, span }
    }

    fn constructor_constructor(
        &mut self,
        id: ItemId,
        ctor: &'scratch parsed::Constructor<'scratch, 'lit>,
    ) -> declared::patterns::Constructor<'scratch, 'lit> {
        let span = ctor.span;
        let node = match &ctor.node {
            parsed::ConstructorNode::Invalid(e) => declared::patterns::ConstructorNode::Invalid(*e),

            parsed::ConstructorNode::Constructor((affix, name), params) => {
                let name = self.define_value(id, span, *affix, *name, Namekind::Pattern);
                match name {
                    Ok(name) => declared::patterns::ConstructorNode::Constructor(name, params),
                    Err(e) => declared::patterns::ConstructorNode::Invalid(e),
                }
            }
        };

        declared::patterns::Constructor { node, span }
    }
}
