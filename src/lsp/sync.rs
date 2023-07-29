use lsp_types::Url;
use nml_compiler::source::SourceId;

use super::Server;

impl Server {
    pub fn insert_document(&mut self, name: Url, text: String) -> SourceId {
        if let Some(source) = self.tracked.get_mut(&name) {
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
