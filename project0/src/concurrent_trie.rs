use std::{
    collections::{hash_map::Entry, HashMap},
    ptr::null_mut,
};

use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

pub type ReadGuard<'a, T> = MappedRwLockReadGuard<'a, T>;

#[derive(Debug)]
pub struct Trie<T: Send + Sync> {
    top_level_nodes: RwLock<HashMap<char, TrieNode<T>>>,
}

unsafe impl<T: Send + Sync> Send for Trie<T> {}
unsafe impl<T: Send + Sync> Sync for Trie<T> {}

impl<T: Send + Sync> Default for Trie<T> {
    fn default() -> Self {
        Self {
            top_level_nodes: RwLock::new(HashMap::new()),
        }
    }
}

impl<T: Send + Sync> Trie<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> ReadGuard<'_, Option<T>> {
        let top_level_nodes = self.top_level_nodes.read();

        RwLockReadGuard::map(top_level_nodes, |map: &HashMap<char, TrieNode<T>>| {
            let key_iter = key.chars();
            let mut map = map;
            let mut output = &None;
            for key in key_iter {
                if let Some(node) = map.get(&key) {
                    map = &node.child_nodes;
                    output = &node.value;
                } else {
                    return &None;
                }
            }
            output
        })
    }

    pub fn insert(&self, key: &str, value: T) -> Result<(), &'static str> {
        let top_level_nodes = &mut *(self.top_level_nodes.write());
        if key.is_empty() {
            return Err("Key can not be empty");
        }
        let mut key_iter = key.chars();
        let first_key = key_iter.next().unwrap();
        //

        if let Entry::Vacant(e) = top_level_nodes.entry(first_key) {
            let new_trie = TrieNode::new_top_level_node();
            e.insert(new_trie);
        }
        let first_trie_node = top_level_nodes.get_mut(&first_key).unwrap();

        first_trie_node.insert(key_iter, value)
    }

    pub fn remove(&self, key: &str) -> Result<(), &'static str> {
        let top_level_nodes = &mut *(self.top_level_nodes.write());

        // ------------------------------------------------
        // Get node
        if key.is_empty() {
            return Err("Key can not be empty");
        }

        let mut key_iter = key.chars();
        let first_key = key_iter.next().unwrap();

        let mut current_node = if let Some(trie_node) = top_level_nodes.get_mut(&first_key) {
            trie_node
        } else {
            return Err("No corresponding value");
        };

        // Last parent of the node with only only one child not containing any value (or matching node)
        let mut parent_node_ptr_and_key = Some((current_node.parent_node_ptr, first_key));

        for key in key_iter {
            current_node = if let Some(node) = current_node.get_node_mut(&key) {
                node
            } else {
                return Err("No corresponding value");
            };

            if (current_node.is_end() && !current_node.child_nodes.is_empty())
                || current_node.child_nodes.len() > 1
            {
                parent_node_ptr_and_key = None;
            } else if parent_node_ptr_and_key.is_none() {
                parent_node_ptr_and_key = Some((current_node.parent_node_ptr, key));
            }
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

        if parent_node_ptr_and_key.is_none() {
            return Ok(());
        }

        // SAFETY:
        unsafe {
            let parent_node_ptr_and_key = parent_node_ptr_and_key.unwrap();
            if parent_node_ptr_and_key.0.is_null() {
                let _ = top_level_nodes.remove(&parent_node_ptr_and_key.1);
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
    use std::{ops::Deref, sync::Arc};

    use super::*;

    #[test]
    fn read_works() {
        let mut trie = Trie::new();
        trie.insert("hello", 99);
        trie.insert("hello", 23);
        trie.insert("bag", 11);
        trie.insert("bucket", 9);
        let trie = Arc::new(trie);

        for n in 0..20 {
            let trie = trie.clone();
            std::thread::spawn(move || {
                assert_eq!(trie.get("hello").deref(), &Some(99));
                assert_eq!(trie.get("bag").deref(), &Some(11));
                println!(
                    "Value of key `hello` is {} (from thread {})",
                    trie.get("hello").deref().unwrap(),
                    n
                );
            });
        }
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
        assert_eq!(*trie.get("hello"), None);
    }

    #[test]
    fn write_works() {
        let trie = Arc::new(Trie::new());

        trie.insert("hello", 99);

        let trie_thread = trie.clone();
        std::thread::spawn(move || {
            println!("Value of key `hello` is {:?}", trie_thread.get("hello"));
            trie_thread.remove("hello");
        })
        .join();

        assert_eq!(trie.get("hello").deref(), &None);
    }
}
