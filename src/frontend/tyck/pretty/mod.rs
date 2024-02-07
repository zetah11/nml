use std::collections::BTreeMap;

use crate::frontend::errors::ErrorId;
use crate::frontend::names::{Label, Name, Names};

use super::solve::{Level, TypeVar};
use super::types::{Generic, Row, VarKind};
use super::{to_name, Scheme, Type};

pub struct Pretty<'a, 'src> {
    vars: BTreeMap<TypeVar, String>,
    show_levels: bool,
    show_error_id: bool,

    counter: usize,
    names: &'a Names<'src>,
}

impl<'a, 'src> Pretty<'a, 'src> {
    pub fn new(names: &'a Names<'src>) -> Self {
        Self {
            vars: BTreeMap::new(),
            show_levels: false,
            show_error_id: false,
            counter: 0,
            names,
        }
    }

    pub fn build(&mut self) -> Prettifier<'_, 'a, 'src> {
        Prettifier { pretty: self }
    }

    pub fn with_show_levels(self, show_levels: bool) -> Self {
        Self {
            show_levels,
            ..self
        }
    }

    pub fn with_show_error_id(self, show_error_id: bool) -> Self {
        Self {
            show_error_id,
            ..self
        }
    }

    fn name(&mut self, var: TypeVar) -> &str {
        self.vars.entry(var).or_insert_with(|| {
            let name = to_name(self.counter);
            self.counter += 1;
            format!(
                "{}{name}",
                match var.1 {
                    VarKind::Type => "$",
                    VarKind::Row => "%",
                }
            )
        })
    }
}

pub struct Prettifier<'a, 'b, 'src> {
    pretty: &'a mut Pretty<'b, 'src>,
}

impl Prettifier<'_, '_, '_> {
    pub fn scheme(&mut self, scheme: &Scheme) -> String {
        if scheme.params.is_empty() {
            self.ty(scheme.ty)
        } else {
            let subst: BTreeMap<_, _> = scheme
                .params
                .iter()
                .filter(|generic| matches!(generic, Generic::Implicit(_)))
                .enumerate()
                .map(|(idx, generic)| (*generic, format!("'{idx}")))
                .collect();

            self.ty_with_subst(scheme.ty, &subst)
        }
    }

    pub fn ty(&mut self, ty: &Type) -> String {
        self.ty_with_subst(ty, &BTreeMap::new())
    }

    pub fn record(&mut self, row: &Row) -> String {
        self.record_with_subst(row, &BTreeMap::new())
    }

    pub fn label(&self, label: &Label) -> String {
        label.0.name().into()
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
            Type::Apply(Type::Apply(Type::Arrow, t), u) => {
                format!("{} -> {}", self.pipes(t, subst), self.arrow(u, subst))
            }

            t => self.pipes(t, subst),
        }
    }

    fn pipes(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            Type::Variant(row) => self.variant_with_subst(row, subst),
            ty => self.apply(ty, subst),
        }
    }

    fn apply(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            // Special-case function types
            Type::Apply(Type::Apply(Type::Arrow, _), _) => self.simple(ty, subst),

            Type::Apply(t, u) => {
                let t = self.apply(t, subst);
                let u = self.simple(u, subst);
                format!("{t} {u}")
            }

            _ => self.simple(ty, subst),
        }
    }

    fn simple(&mut self, ty: &Type, subst: &BTreeMap<Generic, String>) -> String {
        match ty {
            Type::Invalid(e) => self.error(e),
            Type::Var(var, level) => self.var(var, Some(level)),
            Type::Param(name) => self.param(name, subst),
            Type::Named(name) => self.name(name),
            Type::Unit => "unit".into(),
            Type::Integer => "int".into(),
            Type::Arrow => "(->)".into(),
            Type::Record(row) => self.record_with_subst(row, subst),
            ty => format!("({})", self.arrow(ty, subst)),
        }
    }

    fn record_with_subst(&mut self, row: &Row, subst: &BTreeMap<Generic, String>) -> String {
        let (fields, rest) = self.row(row, Some(":"), subst);
        let fields = fields.join(", ");

        match (fields.is_empty(), rest) {
            (true, None) => "{}".into(),
            (false, None) => format!("{{ {fields} }}"),
            (true, Some(rest)) => format!("{{ | {rest} }}"),
            (false, Some(rest)) => format!("{{ {fields} | {rest} }}"),
        }
    }

    fn variant_with_subst(&mut self, row: &Row, subst: &BTreeMap<Generic, String>) -> String {
        let (fields, rest) = self.row(row, None, subst);
        let fields = fields.join(" | ");

        match (fields.is_empty(), rest) {
            (true, None) => "case end".into(),
            (false, None) => fields,
            (true, Some(rest)) => format!("| {rest}"),
            (false, Some(rest)) => format!("{fields} | {rest}"),
        }
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

    fn name(&mut self, name: &Name) -> String {
        let ident = self.pretty.names.get_name(name).name;
        ident.name().into()
    }

    fn param(&mut self, name: &Generic, subst: &BTreeMap<Generic, String>) -> String {
        if let Generic::Ticked(name) = name {
            self.name(name)
        } else {
            subst
                .get(name)
                .expect("attempted to pretty-print unbound generic")
                .clone()
        }
    }
}
