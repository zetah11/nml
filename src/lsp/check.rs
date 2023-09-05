use nml_compiler::alloc::Bump;
use nml_compiler::names::Names;
use nml_compiler::parse::parse;
use nml_compiler::resolve::resolve;
use nml_compiler::source::Source;
use nml_compiler::trees::inferred;
use nml_compiler::tyck;

use super::Server;

impl Server {
    pub fn check_source<'a, 'lit>(
        &'lit self,
        names: &'a Names<'lit>,
        alloc: &'a Bump,
        source: &Source,
    ) -> inferred::Program<'a, 'lit> {
        let parsed = parse(alloc, names, &self.literals, source);
        let resolved = resolve(names, alloc, &parsed);
        let inferred = tyck::infer(alloc, names, &resolved);
        inferred
    }
}
