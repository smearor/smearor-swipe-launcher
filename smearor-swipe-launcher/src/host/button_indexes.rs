/// A list of physical button indexes belonging to a span group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonIndexes(Vec<u8>);

impl ButtonIndexes {
    pub fn new(indexes: Vec<u8>) -> Self {
        Self(indexes)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, u8> {
        self.0.iter()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<u8>> for ButtonIndexes {
    fn from(indexes: Vec<u8>) -> Self {
        Self(indexes)
    }
}

/// A span group identified by name with its associated physical button indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanGroup {
    /// The span group name.
    pub name: String,
    /// All physical button indexes belonging to this span group.
    pub button_indexes: ButtonIndexes,
}
