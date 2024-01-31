//! At the command line, the compiler is mostly used as a "batch" compiler - run
//! occasionally, parsing, checking, and building in one go.

use std::path::Path;

use crate::frontend::alloc::Bump;
use crate::frontend::names::Names;
use crate::frontend::parse::parse;
use crate::frontend::resolve::resolve;
use crate::frontend::source::Sources;
use crate::frontend::tyck::infer;

pub fn run(path: &Path) -> Result<(), BatchError> {
    let file = std::fs::read_to_string(path)?;
    let sources = Sources::new();
    let source = sources.add(file);

    let alloc = Bump::new();
    let names = Names::new();

    let parsed = parse(&alloc, &names, &source);
    let resolved = resolve(&names, &alloc, &parsed);
    let result = infer(&alloc, &names, &resolved);
    let result = result.errors;

    if result.is_perfect() {
        Ok(())
    } else {
        Err(BatchError::CompilerError {
            num_errors: result.num_errors(),
            num_warnings: result.num_warnings(),
        })
    }
}

pub enum BatchError {
    IoError(std::io::Error),
    CompilerError {
        num_errors: usize,
        num_warnings: usize,
    },
}

impl From<std::io::Error> for BatchError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}
