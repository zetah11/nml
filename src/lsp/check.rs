use nml_compiler::alloc::Bump;
use nml_compiler::errors::Errors;
use nml_compiler::names::Names;
use nml_compiler::parse::parse;
use nml_compiler::resolve::{declare, resolve};
use nml_compiler::source::Source;
use nml_compiler::tyck;

use super::Server;

impl Server {
    pub fn check_source(&self, source: &Source) -> Errors {
        let alloc = Bump::new();
        let names = Names::new(&self.idents);

        let parsed = parse(&alloc, &names, source);
        let declared = declare(&alloc, &names, &parsed);
        let resolved = resolve(&names, &alloc, &declared);
        tyck::infer(&names, &resolved)
    }
}
