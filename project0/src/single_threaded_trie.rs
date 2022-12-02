use std::{
    collections::{hash_map::Entry, HashMap},
    ptr::null_mut,
};

#[derive(Debug)]
pub struct Trie<T> {
    top_level_nodes: HashMap<char, TrieNode<T>>,
}

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self {
            top_level_nodes: HashMap::new(),
        }
    }
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        let key_iter = key.chars();
        let mut map = &self.top_level_nodes;
        let mut output = None;
        for key in key_iter {
            if let Some(node) = map.get(&key) {
                map = &node.child_nodes;
                output = node.value.as_ref();
            } else {
                return None;
            }
        }
        output
    }

    pub fn insert(&mut self, key: &str, value: T) -> Result<(), &'static str> {
        if key.is_empty() {
            return Err("Key can not be empty");
        }
        let mut key_iter = key.chars();
        let first_key = key_iter.next().unwrap();
        //

        if let Entry::Vacant(e) = self.top_level_nodes.entry(first_key) {
            let new_trie = TrieNode::new_top_level_node();
            e.insert(new_trie);
        }
        let first_trie_node = self.top_level_nodes.get_mut(&first_key).unwrap();

        first_trie_node.insert(key_iter, value)
    }

    pub fn remove(&mut self, key: &str) -> Result<(), &'static str> {
        // ------------------------------------------------
        // Get node
        if key.is_empty() {
            return Err("Key can not be empty");
        }

        let mut key_iter = key.chars();
        let first_key = key_iter.next().unwrap();

        let mut current_node = if let Some(trie_node) = self.top_level_nodes.get_mut(&first_key) {
            trie_node
        } else {
            return Err("No corresponding value");
        };

        for key in key_iter {
            current_node = if let Some(node) = current_node.get_node_mut(&key) {
                node
            } else {
                return Err("No corresponding value");
            };
        }

        // ------------------------------------------------
        // Removing

        if !current_node.is_end() {
            return Err("No corresponding value");
        }

        if !current_node.child_nodes.is_empty() {
            current_node.set_value(None);
            return Ok(());
        }

        // The key_iter is certain not get exhausted before reaching the top of the `Trie`
        let mut key_iter = key.chars().rev();
        let key = key_iter.next().unwrap();

        // Last parent of the node with only only one child not containing any value (or matching node)
        let mut parent_node_ptr_and_key = (current_node.parent_node_ptr, key);

        for key in key_iter {
            // SAFETY: No mutation of contents of pointer
            unsafe {
                let parent_node = &*(parent_node_ptr_and_key.0);

                if parent_node.child_nodes.len() > 1 || parent_node.is_end() {
                    break;
                }

                parent_node_ptr_and_key = (parent_node.parent_node_ptr, key);
            }
        }

        // SAFETY:
        unsafe {
            if parent_node_ptr_and_key.0.is_null() {
                let _ = self.top_level_nodes.remove(&parent_node_ptr_and_key.1);
            } else {
                let parent_node = &mut *parent_node_ptr_and_key.0;
                let _ = parent_node.child_nodes.remove(&parent_node_ptr_and_key.1);
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct TrieNode<T> {
    value: Option<T>,
    child_nodes: HashMap<char, TrieNode<T>>,
    parent_node_ptr: *mut TrieNode<T>,
}

impl<T> TrieNode<T> {
    fn is_end(&self) -> bool {
        self.value.is_some()
    }

    fn new(parent_node_ptr: *mut TrieNode<T>) -> Self {
        TrieNode {
            value: None,
            child_nodes: HashMap::new(),
            parent_node_ptr,
        }
    }

    fn new_top_level_node() -> Self {
        TrieNode {
            value: None,
            child_nodes: HashMap::new(),
            parent_node_ptr: null_mut(),
        }
    }

    fn as_ptr(&mut self) -> *mut TrieNode<T> {
        self as *mut _
    }

    fn insert<I: Iterator<Item = char>>(
        &mut self,
        key_iter: I,
        value: T,
    ) -> Result<(), &'static str> {
        // Base case
        let mut current_trie = self;

        for key in key_iter {
            let node_pointer = current_trie.as_ptr();
            if let Entry::Vacant(e) = current_trie.child_nodes.entry(key) {
                let new_trie = TrieNode::new(node_pointer);
                e.insert(new_trie);
            }
            let trie = current_trie.child_nodes.get_mut(&key).unwrap();
            current_trie = trie;
        }
        if current_trie.is_end() {
            Err("Duplicate keys are not allowed")
        } else {
            current_trie.value = Some(value);
            Ok(())
        }
    }

    fn set_value(&mut self, value: Option<T>) {
        self.value = value;
    }

    fn get_node_mut(&mut self, key: &char) -> Option<&mut TrieNode<T>> {
        self.child_nodes.get_mut(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_single_value() {
        // Create new `Trie`
        let mut trie = Trie::new();

        // Insert value
        assert!(trie.insert("hello", 99).is_ok());

        // Get value
        assert_eq!(trie.get("hello"), Some(&99));

        // Delete value
        assert!(trie.remove("hello").is_ok());

        // Get value
        assert_eq!(trie.get("hello"), None);
    }

    #[test]
    fn insert_multiple_values() {
        // Create new `Trie`
        let mut trie = Trie::new();

        // Insert values
        assert!(trie.insert("hello", 1).is_ok());
        assert!(trie.insert("hell", 2).is_ok());
        assert!(trie.insert("hel", 3).is_ok());
        assert!(trie.insert("hey", 4).is_ok());
        assert!(trie.insert("back", 4).is_ok());

        println!("{:?}", &trie);

        // Get values
        assert_eq!(trie.get("hello"), Some(&1));
        assert_eq!(trie.get("hell"), Some(&2));
        assert_eq!(trie.get("hel"), Some(&3));
        assert_eq!(trie.get("hey"), Some(&4));

        // Delete value
        assert!(trie.remove("hello").is_ok());
        assert!(trie.remove("hello").is_err());
        assert!(trie.remove("hell").is_ok());
        assert!(trie.remove("hel").is_ok());
        println!("{:?}", &trie);
        assert!(trie.remove("hey").is_ok());
        println!("{:?}", &trie);
        assert!(trie.remove("back").is_ok());
        println!("{:?}", &trie);

        // Get value
        assert_eq!(trie.get("hello"), None);
    }

    #[test]
    fn insert_multiple_values_with_same_key() {
        // Create new `Trie`
        let mut trie = Trie::new();

        // Insert multiple values with same key
        assert!(trie.insert("hello", 1).is_ok());
        assert!(trie.insert("hello", 5).is_err());
    }
}
