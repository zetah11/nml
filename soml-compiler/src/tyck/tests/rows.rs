use super::Store;

#[test]
fn fields() {
    // r => (let a = r.x in let b = r.y in a)
    // --> { x: '1, y: '2 | '3 } -> '1
    Store::with(|s, mut checker| {
        let body = s.var("a");
        let bound = s.field(s.var("r"), "y");
        let body = s.let_in("b", bound, body);
        let bound = s.field(s.var("r"), "x");
        let inner = s.let_in("a", bound, body);
        let expr = s.lambda("r", inner);

        let xt = checker.fresh();
        let expected = s.extend([("x", xt), ("y", checker.fresh())], Some(checker.fresh_row()));
        let expected = s.arrow(expected, xt);

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn overwrite() {
    // r => { x = 5 | r \ x }
    // --> { x: '1 | '2 } -> { x: int | '2 }
    Store::with(|s, mut checker| {
        let old = s.restrict(s.var("r"), "x");
        let body = s.record([("x", s.num(5))], Some(old));
        let expr = s.lambda("r", body);

        let rest = checker.fresh_row();
        let t = s.extend([("x", checker.fresh())], Some(rest));
        let u = s.extend([("x", s.int())], Some(rest));
        let expected = s.arrow(t, u);

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn sneakily_recursive() {
    // r => if True then { x = 2 | r } else { y = 2 | r }
    // --> [error, branches do not unify]

    // The types have a common tail but a distinct prefix, which implies that
    // they are incompatible. Test case taken from
    // "Extensible Records with Scoped Labels" (Daan Leijen, 2005) at
    // https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/scopedlabels.pdf
    Store::with(|s, mut checker| {
        let then = s.record([("x", s.num(2))], Some(s.var("r")));
        let elze = s.record([("y", s.num(2))], Some(s.var("r")));
        let cond = s.if_then(s.bool(true), then, elze);
        let expr = s.lambda("r", cond);

        let _actual = checker.infer(expr);
        assert_eq!(checker.errors.num_errors(), 1);
    });
}

#[test]
fn record_literal() {
    // { x = 1, y = f => f true }
    // --> { x: int, y: (bool -> '1) -> '1 }
    Store::with(|s, mut checker| {
        let lit = s.num(1);
        let lambda = s.lambda("f", s.apply(s.var("f"), s.bool(true)));
        let expr = s.record([("x", lit), ("y", lambda)], None);

        let a = checker.fresh();
        let lit_ty = s.int();
        let lambda_ty = s.arrow(s.arrow(s.boolean(), a), a);
        let expected = s.extend([("x", lit_ty), ("y", lambda_ty)], None);

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    })
}

#[test]
fn single_variant() {
    // x => Test x
    // --> '1 -> Test '1 | '2
    Store::with(|s, mut checker| {
        let expr = s.lambda("x", s.apply(s.variant("Test"), s.var("x")));

        let xt = checker.fresh();
        let expected = s.sum([("Test", xt)], Some(checker.fresh_row()));
        let expected = s.arrow(xt, expected);

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn exhaustive_case() {
    // case A 5 | A x -> x | B y -> 0 end
    // --> int
    Store::with(|s, mut checker| {
        let scrutinee = s.apply(s.variant("A"), s.num(5));
        let case1 = (s.deconstruct("A", s.bind("x")), s.var("x"));
        let case2 = (s.deconstruct("B", s.wildcard()), s.num(5));
        let expr = s.case(scrutinee, [case1, case2]);

        let expected = s.int();

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn wildcard_case() {
    // x => case x | A y -> y | _ -> 5 end
    // --> A int | '1 -> int
    Store::with(|s, mut checker| {
        let case1 = (s.deconstruct("A", s.bind("y")), s.var("y"));
        let case2 = (s.wildcard(), s.num(5));
        let case = s.case(s.var("x"), [case1, case2]);
        let expr = s.lambda("x", case);

        let rt = checker.fresh_row();
        let ret = s.int();
        let arg = s.sum([("A", ret)], Some(rt));
        let expected = s.arrow(arg, ret);

        let actual = checker.infer(expr);

        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn wildcard_in_exhaustive_case() {
    // x => case x | A (M y) -> y | A _ -> 5 | B z -> z end
    // --> A (M int | '1) | B int -> int

    // The pattern nested in the outer `A` constructors is "open" due to the
    // wildcard, while the outer patterns are exhaustive due to the lack of
    // wildcards.
    Store::with(|s, mut checker| {
        let patn1 = s.deconstruct("A", s.deconstruct("M", s.bind("y")));
        let case1 = (patn1, s.var("y"));
        let case2 = (s.deconstruct("A", s.wildcard()), s.num(5));
        let case3 = (s.deconstruct("B", s.bind("z")), s.var("z"));
        let case = s.case(s.var("x"), [case1, case2, case3]);
        let expr = s.lambda("x", case);

        let rest = checker.fresh_row();
        let ret = s.int();
        let nested = s.sum([("M", ret)], Some(rest));
        let arg = s.sum([("A", nested), ("B", ret)], None);
        let expected = s.arrow(arg, ret);

        let actual = checker.infer(expr);

        checker.assert_alpha_equal(expected, actual);
    });
}
