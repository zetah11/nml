use std::collections::BTreeMap;

use super::solve::TypeVar;
use super::{to_name, Scheme, Type};

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
            Type::Invalid(e) => format!("<{e}>"),

            Type::Var(var, l) => {
                let show_levels = self.pretty.show_levels;
                let name = self.pretty.name(*var);
                if show_levels {
                    format!("{name}/{}", l.as_usize())
                } else {
                    String::from(name)
                }
            }

            Type::Param(name) => name.to_string(),

            Type::Boolean => "bool".into(),
            Type::Integer => "int".into(),

            Type::Record(ty) => {
                let (fields, rest) = self.record(ty);
                let fields = fields.join(", ");
                let rest = rest.map(|rest| format!(" | {rest}")).unwrap_or_default();
                format!("{{ {fields}{rest} }}")
            }

            Type::Empty | Type::Extend(..) => {
                let (fields, rest) = self.record(ty);
                let fields = fields.join(", ");
                let rest = rest.map(|rest| format!(" | {rest}")).unwrap_or_default();
                format!("{{ {fields}{rest} }}")
            }

            ty => self.arrow(ty),
        }
    }

    fn record(&mut self, ty: &Type) -> (Vec<String>, Option<String>) {
        let mut fields = vec![];
        let mut rest = None;
        let mut ty = ty;

        loop {
            match ty {
                Type::Extend(label, field, rest) => {
                    let field = self.arrow(field);
                    fields.push(format!("{label}: {field}"));
                    ty = rest;
                }

                Type::Empty => break,

                ty => {
                    rest = Some(self.arrow(ty));
                    break;
                }
            }
        }

        (fields, rest)
    }
}
