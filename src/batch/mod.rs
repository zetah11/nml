//! At the command line, the compiler is mostly used as a "batch" compiler - run
//! occasionally, parsing, checking, and building in one go.

use std::path::Path;

use anyhow::anyhow;
use nml_compiler::alloc::Bump;
use nml_compiler::intern::ThreadedRodeo;
use nml_compiler::names::Names;
use nml_compiler::parse::parse;
use nml_compiler::resolve::resolve;
use nml_compiler::source::Sources;
use nml_compiler::tyck::infer;

pub fn run(path: &Path) -> anyhow::Result<()> {
    let file = std::fs::read_to_string(path)?;
    let sources = Sources::new();
    let source = sources.add(file);

    let alloc = Bump::new();
    let idents = ThreadedRodeo::new();
    let names = Names::new(&idents);

    let parsed = parse(&alloc, &names, &source);
    let resolved = resolve(&names, &alloc, &parsed);
    let result = infer(&names, &resolved);

    if result.is_perfect() {
        Ok(())
    } else {
        Err(anyhow!("{} errors and {} warnings", result.num_errors(), result.num_warnings()))
    }
}
