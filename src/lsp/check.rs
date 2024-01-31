use crate::frontend::alloc::Bump;
use crate::frontend::names::Names;
use crate::frontend::parse::parse;
use crate::frontend::resolve::resolve;
use crate::frontend::source::Source;
use crate::frontend::trees::inferred;
use crate::frontend::tyck;

use super::Server;

impl Server {
    pub fn check_source<'a, 'src>(
        &'src self,
        names: &'a Names<'src>,
        alloc: &'a Bump,
        source: &'src Source,
    ) -> inferred::Program<'a, 'src> {
        let parsed = parse(alloc, names, source);
        let resolved = resolve(names, alloc, &parsed);
        let inferred = tyck::infer(alloc, names, &resolved);
        inferred
    }
}
