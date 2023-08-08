use nml_compiler::alloc::Bump;
use nml_compiler::names::Names;
use nml_compiler::parse::parse;
use nml_compiler::resolve::resolve;
use nml_compiler::source::Source;
use nml_compiler::trees::inferred;
use nml_compiler::tyck;

use super::Server;

impl Server {
    pub fn check_source<'a>(
        &self,
        alloc: &'a Bump,
        source: &Source,
    ) -> (Names, inferred::Program<'a>) {
        let names = Names::new(&self.idents);

        let parsed = parse(alloc, &names, source);
        let resolved = resolve(&names, alloc, &parsed);
        let inferred = tyck::infer(alloc, &names, &resolved);

        (names, inferred)
    }
}
