use nml_compiler::source::SourceId;
use tower_lsp::lsp_types::Url;

use super::Server;

impl Server {
    pub fn insert_document(&self, name: Url, text: String) -> SourceId {
        if let Some(mut source) = self.tracked.get_mut(&name) {
            source.content = text;
            source.id
        } else {
            let source = self.sources.add(text);
            let id = source.id;
            self.names.insert(source.id, name.clone());
            self.tracked.insert(name, source);
            id
        }
    }
}
