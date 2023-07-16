use std::collections::BTreeMap;

use lasso::ThreadedRodeo;

use crate::errors::ErrorId;
use crate::names::{Ident, Label};

use super::solve::{Level, TypeVar};
use super::types::{Generic, Row};
use super::{to_name, Scheme, Type};

#[derive(Debug)]
pub struct Pretty<'ids> {
    vars: BTreeMap<TypeVar, String>,
    show_levels: bool,
    show_error_id: bool,

    counter: usize,
    idents: &'ids ThreadedRodeo<Ident>,
}

impl<'ids> Pretty<'ids> {
    pub fn new(idents: &'ids ThreadedRodeo<Ident>) -> Self {
        Self { vars: BTreeMap::new(), show_levels: false, show_error_id: false, counter: 0, idents }
    }

    pub fn build(&mut self) -> Prettifier<'_, 'ids> {
        Prettifier { pretty: self }
    }

    pub fn with_show_levels(self, show_levels: bool) -> Self {
        Self { show_levels, ..self }
    }

    pub fn with_show_error_id(self, show_error_id: bool) -> Self {
        Self { show_error_id, ..self }
    }

    fn name(&mut self, var: TypeVar) -> &str {
        self.vars.entry(var).or_insert_with(|| {
            let name = to_name(self.counter);
            self.counter += 1;
            format!("'{}", name)
        })
    }
}

pub struct Prettifier<'a, 'ids> {
    pretty: &'a mut Pretty<'ids>,
}

impl Prettifier<'_, '_> {
    pub fn scheme(&mut self, scheme: &Scheme) -> String {
        if scheme.params.is_empty() {
            self.ty(scheme.ty)
        } else {
            let subst: BTreeMap<_, _> = scheme
                .params
                .iter()
                .enumerate()
                .map(|(idx, generic)| (*generic, to_name(idx)))
                .collect();

            let params: Vec<_> = subst.values().cloned().collect();

            format!("for {}. {}", params.join(" "), self.ty_with_subst(scheme.ty, &subst))
        }
    }

    pub fn ty(&mut self, ty: &Type) -> String {
        self.ty_with_subst(ty, &BTreeMap::new())
    }

    pub fn record(&mut self, row: &Row) -> String {
        self.record_with_subst(row, &BTreeMap::new())
    }

    pub fn label(&self, label: &Label) -> String {
        self.pretty.idents.resolve(&label.0).into()
    }

    pub fn var(&mut self, var: &TypeVar, level: Option<&Level>) -> String {
        let show_levels = self.pretty.show_levels;
        let name = self.pretty.name(*var);

        if let Some(level) = level {
            if show_levels {
                return format!("{name}/{}", level.as_usize());
            }
        }

        String::from(name)
    }

    fn ty_with_subst(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        self.arrow(ty, subst)
    }

    fn arrow(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            Type::Fun(t, u) => {
                format!("{} -> {}", self.pipes(t, subst), self.arrow(u, subst))
            }

            t => self.pipes(t, subst),
        }
    }

    fn pipes(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            Type::Variant(row) => self.variant_with_subst(row, subst),
            ty => self.simple(ty, subst),
        }
    }

    fn simple(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            Type::Invalid(e) => self.error(e),
            Type::Var(var, level) => self.var(var, Some(level)),
            Type::Param(name) => self.param(name, subst),
            Type::Named(_name) => "<name>".into(),
            Type::Boolean => "bool".into(),
            Type::Integer => "int".into(),
            Type::Record(row) => self.record_with_subst(row, subst),
            ty => format!("({})", self.arrow(ty, subst)),
        }
    }

    fn record_with_subst(&mut self, row: &Row, subst: &BTreeMap<Generic, String>) -> String {
        let (fields, rest) = self.row(row, Some(":"), subst);
        let fields = fields.join(", ");
        let rest = rest.map(|rest| format!(" | {rest}")).unwrap_or_default();
        format!("{{ {fields}{rest} }}")
    }

    fn variant_with_subst(&mut self, row: &Row, subst: &BTreeMap<Generic, String>) -> String {
        let (fields, rest) = self.row(row, None, subst);

        let rest = if fields.is_empty() { rest } else { rest.map(|rest| format!(" | {rest}")) }
            .unwrap_or_default();
        let fields = fields.join(" | ");

        format!("{fields}{rest}")
    }

    fn row(
        &mut self,
        row: &Row,
        sep: Option<&str>,
        subst: &BTreeMap<Generic, String>,
    ) -> (Vec<String>, Option<String>) {
        let mut fields = vec![];
        let mut rest = None;
        let mut row = row;

        loop {
            match row {
                Row::Extend(label, field, rest) => {
                    let (field, sep) = if let Some(sep) = sep {
                        (self.arrow(field, subst), sep)
                    } else {
                        (self.simple(field, subst), "")
                    };

                    fields.push(format!("{}{sep} {field}", self.label(label)));
                    row = rest;
                }

                Row::Empty => break,

                Row::Invalid(e) => {
                    rest = Some(self.error(e));
                    break;
                }

                Row::Var(var, level) => {
                    rest = Some(self.var(var, Some(level)));
                    break;
                }

                Row::Param(name) => {
                    rest = Some(self.param(name, subst));
                    break;
                }
            }
        }

        (fields, rest)
    }

    fn error(&mut self, e: &ErrorId) -> String {
        if self.pretty.show_error_id {
            format!("<error {}>", e.as_usize())
        } else {
            "<error>".into()
        }
    }

    fn param(&mut self, name: &Generic, subst: &BTreeMap<Generic, String>) -> String {
        subst.get(name).expect("attempted to pretty-print unbound generic").clone()
    }
}
