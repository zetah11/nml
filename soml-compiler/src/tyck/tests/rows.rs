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
        let expected = s.extend([("x", xt), ("y", checker.fresh())], checker.fresh());
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

        let rest = checker.fresh();
        let t = s.extend([("x", checker.fresh())], rest);
        let u = s.extend([("x", s.int())], rest);
        let expected = s.arrow(t, u);

        let actual = checker.infer(expr);

        checker.assert_alpha_equal(expected, actual);
    });
}
