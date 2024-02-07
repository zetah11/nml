use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd)]
pub struct Identifier<'src> {
    name: &'src str,

    /// The hash of the name above; used to quickly reject comparisons and
    /// provide constant time hashing of idents
    hash: u64,
}

impl<'src> Identifier<'src> {
    pub fn new(name: &'src str) -> Self {
        let hash = {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            hasher.finish()
        };

        Self { name, hash }
    }

    pub fn name(&self) -> &'src str {
        self.name
    }
}

impl Hash for Identifier<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

impl PartialEq for Identifier<'_> {
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            return false;
        }

        self.name == other.name
    }
}
