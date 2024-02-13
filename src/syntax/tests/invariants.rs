use proptest::{prop_assert_eq, proptest};

use crate::syntax::parse::parse;

proptest! {
    #[test]
    fn tree_invariants(s in r".*") {
        let node = parse(&s);

        node.check_invariants();
        prop_assert_eq!(s.len(), node.width);
    }
}
