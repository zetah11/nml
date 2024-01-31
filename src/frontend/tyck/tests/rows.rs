use super::Store;

#[test]
fn fields() {
    // r => (let a = r.x in let b = r.y in a)
    // --> { x: '1, y: '2 | '3 } -> '1
    Store::with(|s, mut checker| {
        let body = s.var("a");
        let bound = s.field(s.var("r"), "y");
        let body = s.let_in(s.bind("b"), bound, body);
        let bound = s.field(s.var("r"), "x");
        let inner = s.let_in(s.bind("a"), bound, body);
        let expr = s.lambda(s.bind("r"), inner);

        let xt = checker.fresh();
        let expected = s.extend(
            [("x", xt), ("y", checker.fresh())],
            Some(checker.fresh_row()),
        );
        let expected = s.arrow(expected, xt);

        let actual = checker.infer(&expr);
        checker.assert_alpha_equal(expected, actual.ty);
    });
}

#[test]
fn overwrite() {
    // r => { x = 5 | r \ x }
    // --> { x: '1 | '2 } -> { x: int | '2 }
    Store::with(|s, mut checker| {
        let old = s.restrict(s.var("r"), "x");
        let body = s.record([("x", s.num("5"))], Some(old));
        let expr = s.lambda(s.bind("r"), body);

        let rest = checker.fresh_row();
        let t = s.extend([("x", checker.fresh())], Some(rest));
        let u = s.extend([("x", s.int())], Some(rest));
        let expected = s.arrow(t, u);

        let actual = checker.infer(&expr);
        checker.assert_alpha_equal(expected, actual.ty);
    });
}

#[test]
fn sneakily_recursive() {
    // r => case 0 | a => { x = 2 | r } | b => { y = 2 | r } end
    // --> [error, branches do not unify]

    // The types have a common tail but a distinct prefix, which implies that
    // they are incompatible. Test case taken from
    // "Extensible Records with Scoped Labels" (Daan Leijen, 2004) at
    // https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/scopedlabels.pdf
    Store::with(|s, mut checker| {
        let arm1 = s.record([("x", s.num("2"))], Some(s.var("r")));
        let arm2 = s.record([("y", s.num("2"))], Some(s.var("r")));

        let pat1 = s.bind("a");
        let pat2 = s.bind("b");

        let case = s.case(s.num("0"), [(pat1, arm1), (pat2, arm2)]);
        let expr = s.lambda(s.bind("r"), case);

        let _actual = checker.infer(&expr);
        assert_eq!(checker.errors.num_errors(), 1);
    });
}

#[test]
fn record_literal() {
    // { x = 1, y = f => f 0 }
    // --> { x: int, y: (int -> '1) -> '1 }
    Store::with(|s, mut checker| {
        let lit = s.num("1");
        let lambda = s.lambda(s.bind("f"), s.apply(s.var("f"), s.num("0")));
        let expr = s.record([("x", lit), ("y", lambda)], None);

        let a = checker.fresh();
        let lit_ty = s.int();
        let lambda_ty = s.arrow(s.arrow(s.int(), a), a);
        let expected = s.extend([("x", lit_ty), ("y", lambda_ty)], None);

        let actual = checker.infer(&expr);
        checker.assert_alpha_equal(expected, actual.ty);
    })
}
