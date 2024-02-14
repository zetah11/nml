//! The main invariant for the lexer is that no token spans multiple lines. We
//! can check this by parsing the entire string at once, and on a line-by-line
//! basis, and check that we get the exact same result.

use proptest::prelude::prop;
use proptest::strategy::Strategy;
use proptest::{prop_assert_eq, proptest};

use crate::syntax::parse::tokenize;

proptest! {
    #[test]
    fn line_independence(lines in lines()) {
        let lines_tokens: Vec<_> = lines.iter().flat_map(|line| tokenize(line)).collect();

        let source = lines.join("");
        let source_tokens: Vec<_> = tokenize(&source).collect();

        prop_assert_eq!(lines_tokens, source_tokens);
    }
}

/// A strategy for generating a vec of arbitrary strings with one or more
/// newlines at the end.
fn lines() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(r".*\n+", 0..50)
}
