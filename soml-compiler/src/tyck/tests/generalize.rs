use super::Store;
use crate::tyck::Checker;

#[test]
fn identity() {
    // let id = x => x in let y = id 5 in id true
    // --> bool
    Store::with(|s| {
        let lam = s.lambda("x", s.var("x"));
        let body = s.apply(s.var("id"), s.bool(true));
        let bound = s.apply(s.var("id"), s.num(5));
        let body = s.bind("y", bound, body);
        let expr = s.bind("id", lam, body);

        let expected = s.boolean();

        let mut checker = Checker::new(s.types);
        let actual = checker.infer(expr);
        let actual = checker.apply(actual);

        checker.assert_alpha_equal(expected, actual);
    });
}

#[test]
fn nested() {
    // let f = x => (let y = z => x in y) in f 5
    // --> '1 -> int
    Store::with(|s| {
        let inner = s.lambda("z", s.var("x"));
        let inner = s.bind("y", inner, s.var("y"));
        let lambda = s.lambda("x", inner);
        let expr = s.bind("f", lambda, s.apply(s.var("f"), s.num(5)));

        let mut checker = Checker::new(s.types);

        let expected = s.arrow(checker.fresh(), s.int());

        let actual = checker.infer(expr);
        let actual = checker.apply(actual);

        checker.assert_alpha_equal(expected, actual);
    });
}
