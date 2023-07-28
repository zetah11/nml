use nml_compiler::alloc::Bump;
use nml_compiler::names::Names;
use nml_compiler::parse::parse;
use nml_compiler::resolve::resolve;
use nml_compiler::source::Source;
use nml_compiler::tyck;

use super::Server;

impl Server {
    pub async fn check_source(&self, source: &Source) {
        let alloc = Bump::new();
        let names = Names::new(&self.idents);

        let parsed = parse(&alloc, &names, source);
        let resolved = resolve(&names, &alloc, &parsed);
        let mut errors = tyck::infer(&names, &resolved);

        self.send_diagnostics(&mut errors).await;
    }
}
