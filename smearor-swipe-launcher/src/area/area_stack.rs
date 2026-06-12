use std::collections::VecDeque;

/// Manages a stack of areas for nested sub-menus
pub struct AreaStack {
    /// Stack of areas, with the top being the most recently added
    stack: VecDeque<String>,
}

impl AreaStack {
    /// Create a new empty AreaStack
    pub fn new() -> Self {
        Self { stack: VecDeque::new() }
    }

    /// Push an area onto the stack
    pub fn push(&mut self, area_id: String) {
        self.stack.push_back(area_id);
    }

    /// Pop the top area from the stack
    pub fn pop(&mut self) -> Option<String> {
        self.stack.pop_back()
    }

    /// Peek at the top area without removing it
    pub fn peek(&self) -> Option<&String> {
        self.stack.back()
    }

    /// Get the number of areas in the stack
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Check if an area is in the stack
    pub fn contains(&self, area_id: &str) -> bool {
        self.stack.iter().any(|id| id == area_id)
    }

    /// Remove a specific area from the stack
    pub fn remove(&mut self, area_id: &str) -> bool {
        if let Some(pos) = self.stack.iter().position(|id| id == area_id) {
            self.stack.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all areas from the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get all area IDs in the stack (bottom to top)
    pub fn get_all(&self) -> Vec<&String> {
        self.stack.iter().collect()
    }
}

impl Default for AreaStack {
    fn default() -> Self {
        Self::new()
    }
}
