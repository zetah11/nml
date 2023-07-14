pub use self::vars::Level;

mod rows;
mod vars;

use std::collections::BTreeMap;

use log::trace;

use super::memory::Alloc;
use super::pretty::Prettifier;
use super::tree::RecordRow;
use super::{to_name, ErrorId, Name, Scheme, Type};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeVar(usize);

pub struct Solver<'a> {
    subst: BTreeMap<TypeVar, &'a Type<'a>>,
    record_subst: BTreeMap<TypeVar, &'a RecordRow<'a>>,
    counter: usize,
    level: usize,
}

impl<'a> Solver<'a> {
    pub fn new() -> Self {
        Self {
            subst: BTreeMap::new(),
            record_subst: BTreeMap::new(),
            counter: 0,
            level: 0,
        }
    }

    pub fn fresh(&mut self, alloc: &'a Alloc<'a>) -> &'a Type<'a> {
        self.counter += 1;
        alloc.ty(Type::Var(TypeVar(self.counter), Level::new(self.level)))
    }

    pub fn fresh_record(&mut self, alloc: &'a Alloc<'a>) -> &'a RecordRow<'a> {
        self.counter += 1;
        alloc.record(RecordRow::Var(
            TypeVar(self.counter),
            Level::new(self.level),
        ))
    }

    pub fn enter(&mut self) {
        self.level += 1;
    }

    pub fn exit(&mut self) {
        self.level -= 1;
    }

    fn new_var(&mut self) -> (TypeVar, Level) {
        self.counter += 1;
        (TypeVar(self.counter), Level::new(self.level))
    }
}

/// Apply
impl<'a> Solver<'a> {
    /// Apply the current substitution to the given type.
    pub fn apply(&self, alloc: &'a Alloc<'a>, ty: &'a Type<'a>) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer | Type::Param(_) => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.apply(alloc, ty)
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.apply(alloc, t);
                let u = self.apply(alloc, u);
                alloc.ty(Type::Fun(t, u))
            }

            Type::Record(record) => {
                let record = self.apply_record(alloc, record);
                alloc.ty(Type::Record(record))
            }
        }
    }

    fn apply_record(&self, alloc: &'a Alloc<'a>, record: &'a RecordRow<'a>) -> &'a RecordRow<'a> {
        match record {
            RecordRow::Invalid(_) | RecordRow::Empty | RecordRow::Param(_) => record,

            RecordRow::Var(v, _) => {
                if let Some(record) = self.record_subst.get(v) {
                    self.apply_record(alloc, record)
                } else {
                    record
                }
            }

            RecordRow::Extend(label, field, rest) => {
                let field = self.apply(alloc, field);
                let rest = self.apply_record(alloc, rest);
                alloc.record(RecordRow::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Instantiation
impl<'a> Solver<'a> {
    pub fn instantiate(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        scheme: &Scheme<'a>,
    ) -> &'a Type<'a> {
        trace!(
            "ins {}",
            pretty.scheme(&Scheme {
                params: scheme.params.clone(),
                ty: self.apply(alloc, scheme.ty),
            })
        );

        let subst = scheme
            .params
            .iter()
            .map(|name| (name, self.new_var()))
            .collect();

        self.inst_ty(alloc, &subst, scheme.ty)
    }

    fn inst_ty(
        &self,
        alloc: &'a Alloc<'a>,
        subst: &BTreeMap<&Name, (TypeVar, Level)>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.inst_ty(alloc, subst, ty)
                } else {
                    ty
                }
            }

            Type::Param(n) => subst
                .get(n)
                .map(|(var, level)| alloc.ty(Type::Var(*var, level.clone())))
                .unwrap_or(ty),

            Type::Fun(t, u) => {
                let t = self.inst_ty(alloc, subst, t);
                let u = self.inst_ty(alloc, subst, u);
                alloc.ty(Type::Fun(t, u))
            }

            Type::Record(record) => {
                let record = self.inst_record(alloc, subst, record);
                alloc.ty(Type::Record(record))
            }
        }
    }

    fn inst_record(
        &self,
        alloc: &'a Alloc<'a>,
        subst: &BTreeMap<&Name, (TypeVar, Level)>,
        record: &'a RecordRow<'a>,
    ) -> &'a RecordRow<'a> {
        match record {
            RecordRow::Invalid(_) | RecordRow::Empty => record,

            RecordRow::Var(v, _) => {
                if let Some(record) = self.record_subst.get(v) {
                    self.inst_record(alloc, subst, record)
                } else {
                    record
                }
            }

            RecordRow::Param(n) => subst
                .get(n)
                .map(|(var, level)| alloc.record(RecordRow::Var(*var, level.clone())))
                .unwrap_or(record),

            RecordRow::Extend(label, field, rest) => {
                let field = self.inst_ty(alloc, subst, field);
                let rest = self.inst_record(alloc, subst, rest);
                alloc.record(RecordRow::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Generalization
impl<'a> Solver<'a> {
    pub fn generalize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        ty: &'a Type<'a>,
    ) -> Scheme<'a> {
        trace!("gen {}", pretty.ty(self.apply(alloc, ty)));

        let mut subst = BTreeMap::new();
        let ty = self.gen_ty(alloc, &mut subst, ty);
        let params = subst.into_values().collect();
        Scheme { params, ty }
    }

    fn gen_ty(
        &mut self,
        alloc: &'a Alloc<'a>,
        subst: &mut BTreeMap<TypeVar, Name>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Param(_) | Type::Boolean | Type::Integer => ty,

            Type::Var(v, level) => {
                if let Some(ty) = self.subst.get(v) {
                    self.gen_ty(alloc, subst, ty)
                } else if let Some(param) = subst.get(v) {
                    alloc.ty(Type::Param(param.clone()))
                } else if level.can_generalize(self.level) {
                    let name = Name::new(to_name(v.0));
                    subst.insert(*v, name.clone());
                    alloc.ty(Type::Param(name))
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.gen_ty(alloc, subst, t);
                let u = self.gen_ty(alloc, subst, u);
                alloc.ty(Type::Fun(t, u))
            }

            Type::Record(record) => {
                let record = self.gen_record(alloc, subst, record);
                alloc.ty(Type::Record(record))
            }
        }
    }

    fn gen_record(
        &mut self,
        alloc: &'a Alloc<'a>,
        subst: &mut BTreeMap<TypeVar, Name>,
        record: &'a RecordRow<'a>,
    ) -> &'a RecordRow<'a> {
        match record {
            RecordRow::Invalid(_) | RecordRow::Param(_) | RecordRow::Empty => record,

            RecordRow::Var(v, level) => {
                if let Some(record) = self.record_subst.get(v) {
                    self.gen_record(alloc, subst, record)
                } else if let Some(param) = subst.get(v) {
                    alloc.record(RecordRow::Param(param.clone()))
                } else if level.can_generalize(self.level) {
                    let name = Name::new(to_name(v.0));
                    subst.insert(*v, name.clone());
                    alloc.record(RecordRow::Param(name))
                } else {
                    record
                }
            }

            RecordRow::Extend(label, field, rest) => {
                let field = self.gen_ty(alloc, subst, field);
                let rest = self.gen_record(alloc, subst, rest);
                alloc.record(RecordRow::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Unification
impl<'a> Solver<'a> {
    pub fn unify(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        trace!(
            "uni {}  ~  {}",
            pretty.ty(self.apply(alloc, lhs)),
            pretty.ty(self.apply(alloc, rhs))
        );
        self.unify_ty(pretty, alloc, lhs, rhs)
    }

    fn unify_ty(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        match (lhs, rhs) {
            (Type::Boolean, Type::Boolean) => {}
            (Type::Boolean, Type::Invalid(_)) | (Type::Invalid(_), Type::Boolean) => {}

            (Type::Integer, Type::Integer) => {}
            (Type::Integer, Type::Invalid(_)) | (Type::Invalid(_), Type::Integer) => {}

            (Type::Param(_), Type::Param(_)) => {}
            (Type::Param(_), Type::Invalid(_)) | (Type::Invalid(_), Type::Param(_)) => {}

            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => {
                self.unify_ty(pretty, alloc, t1, t2);
                self.unify_ty(pretty, alloc, u1, u2);
            }
            (Type::Fun(t, u), e @ Type::Invalid(_)) | (e @ Type::Invalid(_), Type::Fun(t, u)) => {
                self.unify_ty(pretty, alloc, t, e);
                self.unify_ty(pretty, alloc, u, e);
            }

            (Type::Record(row1), Type::Record(row2)) => {
                self.unify_record(pretty, alloc, row1, row2)
            }

            (Type::Record(row), Type::Invalid(e)) | (Type::Invalid(e), Type::Record(row)) => {
                let e = alloc.record(RecordRow::Invalid(e.clone()));
                self.unify_record(pretty, alloc, row, e)
            }

            (Type::Var(var, level), ty) | (ty, Type::Var(var, level)) => {
                if let Some(rhs) = self.subst.get(var) {
                    self.unify_ty(pretty, alloc, ty, rhs)
                } else {
                    self.set(alloc, var, level, ty)
                }
            }

            (Type::Invalid(_), Type::Invalid(_)) => {}

            // Use the exhaustiveness check to ensure termination when unifying
            // with error types
            (
                Type::Boolean | Type::Integer | Type::Param(_) | Type::Fun(..) | Type::Record(_),
                Type::Boolean | Type::Integer | Type::Param(_) | Type::Fun(..) | Type::Record(_),
            ) => {
                let e = alloc.ty(Type::Invalid(ErrorId::new("inequal types")));
                self.unify_ty(pretty, alloc, lhs, e);
                self.unify_ty(pretty, alloc, e, rhs);
            }
        }
    }

    fn unify_record(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        lhs: &'a RecordRow<'a>,
        rhs: &'a RecordRow<'a>,
    ) {
        match (lhs, rhs) {
            (RecordRow::Empty, RecordRow::Empty) => {}
            (RecordRow::Empty, RecordRow::Invalid(_))
            | (RecordRow::Invalid(_), RecordRow::Empty) => {}

            (RecordRow::Extend(label, field1, rest1), row2 @ RecordRow::Extend(..)) => {
                let (field2, rest2) = self.rewrite(pretty, alloc, row2, label);
                self.unify_ty(pretty, alloc, field1, field2);
                self.unify_record(pretty, alloc, rest1, rest2);
            }

            (RecordRow::Extend(_, field, rest), e @ RecordRow::Invalid(id))
            | (e @ RecordRow::Invalid(id), RecordRow::Extend(_, field, rest)) => {
                let et = alloc.ty(Type::Invalid(id.clone()));
                self.unify_ty(pretty, alloc, field, et);
                self.unify_record(pretty, alloc, rest, e);
            }

            (RecordRow::Param(n), RecordRow::Param(m)) if n == m => {}
            (RecordRow::Param(_), RecordRow::Invalid(_))
            | (RecordRow::Invalid(_), RecordRow::Param(_)) => {}

            (RecordRow::Var(var, level), record) | (record, RecordRow::Var(var, level)) => {
                if let Some(rhs) = self.record_subst.get(var) {
                    self.unify_record(pretty, alloc, record, rhs)
                } else {
                    self.set_record(alloc, var, level, record)
                }
            }

            (RecordRow::Invalid(_), RecordRow::Invalid(_)) => {}

            (
                RecordRow::Empty | RecordRow::Extend(..) | RecordRow::Param(_),
                RecordRow::Empty | RecordRow::Extend(..) | RecordRow::Param(_),
            ) => {
                let e = alloc.record(RecordRow::Invalid(ErrorId::new("inequal rows")));
                self.unify_record(pretty, alloc, lhs, e);
                self.unify_record(pretty, alloc, e, rhs);
            }
        }
    }

    fn set(&mut self, alloc: &'a Alloc<'a>, var: &TypeVar, level: &Level, ty: &'a Type<'a>) {
        if let Type::Var(v, l2) = ty {
            l2.set_min(level);
            if v == var {
                return;
            }
        }

        // Occurs check
        if self.occurs(var, level, ty) {
            let ty = alloc.ty(Type::Invalid(ErrorId::new("recursive type")));
            return self.set(alloc, var, level, ty);
        }

        let prev = self.subst.insert(*var, ty);
        debug_assert!(prev.is_none(), "overwrote previous unification variable");
    }

    fn set_record(
        &mut self,
        alloc: &'a Alloc<'a>,
        var: &TypeVar,
        level: &Level,
        record: &'a RecordRow<'a>,
    ) {
        if let RecordRow::Var(v, l2) = record {
            l2.set_min(level);
            if v == var {
                return;
            }
        }

        if self.occurs_record(var, level, record) {
            let record = alloc.record(RecordRow::Invalid(ErrorId::new("recursive record")));
            return self.set_record(alloc, var, level, record);
        }

        let prev = self.record_subst.insert(*var, record);
        debug_assert!(
            prev.is_none(),
            "overwrote previous record row unificiation variable"
        );
    }

    fn occurs(&self, var: &TypeVar, l1: &Level, ty: &Type) -> bool {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer | Type::Param(_) => false,

            Type::Var(war, l2) => {
                if let Some(ty) = self.subst.get(war) {
                    self.occurs(var, l1, ty)
                } else {
                    l2.set_min(l1);
                    var == war
                }
            }

            Type::Record(record) => self.occurs_record(var, l1, record),

            Type::Fun(t, u) => self.occurs(var, l1, t) || self.occurs(var, l1, u),
        }
    }

    fn occurs_record(&self, var: &TypeVar, l1: &Level, record: &RecordRow) -> bool {
        match record {
            RecordRow::Invalid(_) | RecordRow::Empty | RecordRow::Param(_) => false,

            RecordRow::Var(war, l2) => {
                if let Some(record) = self.record_subst.get(war) {
                    self.occurs_record(var, l1, record)
                } else {
                    l2.set_min(l1);
                    var == war
                }
            }

            RecordRow::Extend(_, field, rest) => {
                self.occurs(var, l1, field) || self.occurs_record(var, l1, rest)
            }
        }
    }
}
