use std::collections::BTreeMap;

use super::solve::{Level, TypeVar};
use super::tree::RecordRow;
use super::{to_name, ErrorId, Name, Scheme, Type};

#[derive(Debug, Default)]
pub struct Pretty {
    vars: BTreeMap<TypeVar, String>,
    show_levels: bool,
    counter: usize,
}

impl Pretty {
    pub fn build(&mut self) -> Prettifier {
        Prettifier { pretty: self }
    }

    pub fn with_show_levels(self, show_levels: bool) -> Self {
        Self {
            show_levels,
            ..self
        }
    }

    fn name(&mut self, var: TypeVar) -> &str {
        self.vars.entry(var).or_insert_with(|| {
            let name = to_name(self.counter);
            self.counter += 1;
            format!("'{}", name)
        })
    }
}

pub struct Prettifier<'a> {
    pretty: &'a mut Pretty,
}

impl Prettifier<'_> {
    pub fn scheme(&mut self, scheme: &Scheme) -> String {
        if scheme.params.is_empty() {
            self.ty(scheme.ty)
        } else {
            let params: Vec<_> = scheme.params.iter().map(|name| name.to_string()).collect();
            format!("for {}. {}", params.join(" "), self.ty(scheme.ty))
        }
    }

    pub fn ty(&mut self, ty: &Type) -> String {
        self.arrow(ty)
    }

    fn arrow(&mut self, ty: &Type) -> String {
        match ty {
            Type::Fun(t, u) => {
                format!("{} -> {}", self.simple(t), self.arrow(u))
            }

            t => self.simple(t),
        }
    }

    fn simple(&mut self, ty: &Type) -> String {
        match ty {
            Type::Invalid(e) => self.error(e),
            Type::Var(var, level) => self.var(var, level),
            Type::Param(name) => self.param(name),

            Type::Boolean => "bool".into(),
            Type::Integer => "int".into(),

            Type::Record(row) => {
                let (fields, rest) = self.record(row);
                let fields = fields.join(", ");
                let rest = rest.map(|rest| format!(" | {rest}")).unwrap_or_default();
                format!("{{ {fields}{rest} }}")
            }

            ty => format!("({})", self.arrow(ty)),
        }
    }

    fn record(&mut self, row: &RecordRow) -> (Vec<String>, Option<String>) {
        let mut fields = vec![];
        let mut rest = None;
        let mut row = row;

        loop {
            match row {
                RecordRow::Extend(label, field, rest) => {
                    let field = self.arrow(field);
                    fields.push(format!("{label}: {field}"));
                    row = rest;
                }

                RecordRow::Empty => break,

                RecordRow::Invalid(e) => {
                    rest = Some(self.error(e));
                    break;
                }

                RecordRow::Var(var, level) => {
                    rest = Some(self.var(var, level));
                    break;
                }

                RecordRow::Param(name) => {
                    rest = Some(self.param(name));
                    break;
                }
            }
        }

        (fields, rest)
    }

    fn error(&mut self, e: &ErrorId) -> String {
        format!("<{e}>")
    }

    fn var(&mut self, var: &TypeVar, level: &Level) -> String {
        let show_levels = self.pretty.show_levels;
        let name = self.pretty.name(*var);
        if show_levels {
            format!("{name}/{}", level.as_usize())
        } else {
            String::from(name)
        }
    }

    fn param(&mut self, name: &Name) -> String {
        name.to_string()
    }
}
