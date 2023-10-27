use std::collections::BTreeMap;

use crate::trees::{declared, resolved};

use super::{Namespace, Resolver};

impl<'a, 'scratch, 'lit, 'err> Resolver<'a, 'scratch, 'lit, 'err> {
    pub(super) fn declare_item(
        &mut self,
        item: declared::patterns::Item<'scratch, 'lit>,
    ) -> declared::Item<'a, 'scratch, 'lit> {
        let id = item.id;
        let span = item.span;
        let node = match item.node {
            declared::patterns::ItemNode::Invalid(e) => declared::ItemNode::Invalid(e),
            declared::patterns::ItemNode::Let(pattern, expr, ()) => {
                let mut this_scope = BTreeMap::new();
                let spine = self.function_spine(id, &mut this_scope, pattern);
                let spine: declared::Spine<'scratch, 'lit, resolved::Pattern<'a, 'lit>> =
                    spine.map(|pattern| self.pattern(Namespace::Value, &mut this_scope, &pattern));

                declared::ItemNode::Let(spine, expr, this_scope)
            }

            declared::patterns::ItemNode::Data(pattern, body) => {
                let mut gen_scope = BTreeMap::new();
                let spine = self.function_spine(id, &mut gen_scope, pattern);
                let spine =
                    spine.map(|pattern| self.pattern(Namespace::Type, &mut gen_scope, &pattern));

                if !gen_scope.is_empty() {
                    let span = pattern.span;
                    let e = self.errors.name_error(span).implicit_type_var_in_data();
                    declared::ItemNode::Invalid(e)
                } else {
                    declared::ItemNode::Data(spine, body)
                }
            }
        };

        declared::Item { node, span, id }
    }
}
