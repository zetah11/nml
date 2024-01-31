use super::Store;

#[test]
fn nested() {
    // let f = x => (let y = z => x in y) in f 5
    // --> '1 -> int
    Store::with(|s, mut checker| {
        let inner = s.lambda(s.bind("z"), s.var("x"));
        let inner = s.let_in(s.bind("y"), inner, s.var("y"));
        let lambda = s.lambda(s.bind("x"), inner);
        let expr = s.let_in(s.bind("f"), lambda, s.apply(s.var("f"), s.num("5")));

        let expected = s.arrow(checker.fresh(), s.int());

        let actual = checker.infer(&expr);
        checker.assert_alpha_equal(expected, actual.ty);
    });
}
