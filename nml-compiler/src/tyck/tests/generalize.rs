use super::Store;

#[test]
fn identity() {
    // let id = x => x in let y = id 5 in id true
    // --> bool
    Store::with(|s, mut checker| {
        let lam = s.lambda(s.bind("x"), s.var("x"));
        let body = s.apply(s.var("id"), s.bool(true));
        let bound = s.apply(s.var("id"), s.num(5));
        let body = s.let_in("y", bound, body);
        let expr = s.let_in("id", lam, body);

        let expected = s.boolean();

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn nested() {
    // let f = x => (let y = z => x in y) in f 5
    // --> '1 -> int
    Store::with(|s, mut checker| {
        let inner = s.lambda(s.bind("z"), s.var("x"));
        let inner = s.let_in("y", inner, s.var("y"));
        let lambda = s.lambda(s.bind("x"), inner);
        let expr = s.let_in("f", lambda, s.apply(s.var("f"), s.num(5)));

        let expected = s.arrow(checker.fresh(), s.int());

        let actual = checker.infer(expr);
        checker.assert_alpha_equal(expected, actual);
    });
}
