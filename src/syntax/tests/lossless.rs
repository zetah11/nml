//! The parser produces a lossless tree.

use proptest::{prop_assert_eq, proptest};

use crate::syntax::parse::parse;

proptest! {
    #[test]
    fn renders_the_same(s in r".*") {
        let node = parse(&s);
        prop_assert_eq!(s, node.write());
    }
}

#[test]
fn render_smiley_space() {
    let s = "(: ";
    let node = parse(s);
    assert_eq!(s, node.write());
}

#[test]
fn render_smiley_box() {
    let s = "(:𐺰";
    let node = parse(s);
    assert_eq!(s, node.write());
}

#[test]
fn trailing_whitespace() {
    let s = "a ";
    let node = parse(s);
    assert_eq!(s, node.write());
}
