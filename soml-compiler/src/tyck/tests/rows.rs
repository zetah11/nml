use super::Store;

#[test]
fn fields() {
    // r => (let a = r.x in let b = r.y in a)
    // --> { x: '1, y: '2 | '3 } -> '1
    Store::with(|s, mut checker| {
        let body = s.var("a");
        let bound = s.field(s.var("r"), "y");
        let body = s.bind("b", bound, body);
        let bound = s.field(s.var("r"), "x");
        let inner = s.bind("a", bound, body);
        let expr = s.lambda("r", inner);

        let xt = checker.fresh();
        let expected = s.extend([("x", xt), ("y", checker.fresh())], checker.fresh_record());
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
        let body = s.update("x", s.num(5), old);
        let expr = s.lambda("r", body);

        let rest = checker.fresh_record();
        let t = s.extend([("x", checker.fresh())], rest);
        let u = s.extend([("x", s.int())], rest);
        let expected = s.arrow(t, u);

        let actual = checker.infer(expr);

        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn sneakily_recursive() {
    // r => if True then { x = 2 | r } else { y = 2 | r }
    // --> [error, branches do not unify]
    // from "Extensible Records with Scoped Labels" (Daan Leijen, 2005)
    // https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/scopedlabels.pdf
    Store::with(|s, mut checker| {
        let then = s.update("x", s.num(2), s.var("r"));
        let elze = s.update("y", s.num(2), s.var("r"));
        let cond = s.if_then(s.bool(true), then, elze);
        let expr = s.lambda("r", cond);

        let _actual = checker.infer(expr);
        assert_eq!(checker.errors.num_errors(), 1);
    });
}
