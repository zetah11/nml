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
        let actual = checker.apply(actual);

        checker.assert_alpha_equal(expected, actual);
    });
}
