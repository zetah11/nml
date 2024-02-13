//! The parser should be pure and total.

use proptest::proptest;

use crate::syntax::green::{Data, Kind};

use crate::syntax::parse::parse;

proptest! {
    #[test]
    fn doesnt_crash(s in r".*") {
        let _ = parse(&s);
    }
}

#[test]
fn tolerates_tiny_gibberish() {
    let _ = parse("￼0{");
}

#[test]
fn empty_source() {
    let node = parse(r"");
    assert_eq!(0, node.width);
    assert_eq!(Kind::Source, node.kind);
    assert_eq!(Data::Node(Vec::new().into()), node.data);
}

#[test]
fn en_quad() {
    let _ = parse("\u{2000}");
}

#[test]
fn nonsense_in_thing() {
    let _ = parse("(\u{a8ff}");
}
