use crate::frontend::alloc::Bump;
use crate::frontend::names::Names;
use crate::frontend::parse::parse;
use crate::frontend::resolve::resolve;
use crate::frontend::source::Source;
use crate::frontend::trees::inferred;
use crate::frontend::tyck;

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
