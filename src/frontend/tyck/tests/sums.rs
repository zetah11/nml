use super::Store;
use crate::frontend::tyck::Scheme;

#[test]
fn two_sum() {
    // Some: int -> opt    None: opt
    // x => case x | Some y -> y | None -> 5 end
    // --> opt -> int
    Store::with(|s, mut checker| {
        let some = s.name("Some");
        let none = s.name("None");
        let option = s.nominal("opt");
        let some_ty = s.arrow(s.int(), option);

        checker.env.insert(some, Scheme::mono(some_ty));
        checker.env.insert(none, Scheme::mono(option));

        let case1 = (s.apply_pat(s.named("Some"), s.bind("y")), s.var("y"));
        let case2 = (s.named("None"), s.num(5));
        let case = s.case(s.var("x"), [case1, case2]);
        let expr = s.lambda(s.bind("x"), case);

        let expected = s.arrow(option, s.int());

        let actual = checker.infer(&expr);
        checker.assert_alpha_equal(expected, actual.ty);
    });
}
