use std::cmp::Ordering;
use std::collections::btree_map::{Iter, Values};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::n8sim::fs::FsNode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeKey {
    name: String,
    is_dir: bool,
}
impl NodeKey {
    pub fn new(name: impl Into<String>, is_dir: bool) -> Self {
        Self {
            name: name.into(),
            is_dir,
        }
    }
    
    pub fn from(node: &FsNode) -> Self {
        Self {
            name: node.name().to_string(),
            is_dir: node.is_file(),
        }
    }
    
    pub fn assume_dir(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: true,
        }
    }
    
    pub fn assume_file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: false,
        }
    }
}

impl PartialOrd for NodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NodeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_dir, other.is_dir) {
            (true, true) | (false, false) => {
                self.name.to_ascii_uppercase().cmp(&other.name.to_ascii_uppercase())
            },
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortedNodes {
    inner: BTreeMap<NodeKey, FsNode>,
}

impl SortedNodes {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }
    
    pub fn insert(&mut self, node: FsNode) -> Option<FsNode> {
        self.inner.insert(NodeKey::from(&node), node)
    }
    
    pub fn contains(&self, name: impl AsRef<str>) -> bool {
        self.inner.contains_key(&NodeKey::assume_dir(name.as_ref())) || self.inner.contains_key(&NodeKey::assume_file(name.as_ref()))
    }
    
    pub fn modify<T, F: FnOnce(&mut FsNode) -> T>(&mut self, name: impl AsRef<str>, func: F) -> Option<T> {
        let name = name.as_ref();
        
        let mut result = None;
        let mut new_name = None;
        let mut is_dir = true;
        
        let call_func = |ref mut node: &mut FsNode| {
            result = Some(func(node));
            
            if node.name() != name {
                new_name = Some(node.name().to_string());
            }
        };
        
        if let Some(node) = self.inner.get_mut(&NodeKey::assume_dir(name)) {
            call_func(node);
            is_dir = true;
        } else if let Some(node) = self.inner.get_mut(&NodeKey::assume_file(name)) {
            call_func(node);
            is_dir = false;
        }
        
        if let Some(new_name) = new_name {
            let node = self.inner.remove(&NodeKey::new(name, is_dir)).expect("node should still exist");
            self.inner.insert(NodeKey::new(new_name, is_dir), node);
        }
        
        result
    }
    
    pub fn remove(&mut self, name: impl AsRef<str>) -> Option<FsNode> {
        self.inner.remove(&NodeKey::assume_dir(name.as_ref())).or_else(|| self.inner.remove(&NodeKey::assume_file(name.as_ref())))
    }
    
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    pub fn iter<'a>(&'a self) -> Values<'a, NodeKey, FsNode> {
        self.inner.values()
    }
}

impl<'a> IntoIterator for &'a SortedNodes {
    type Item = (&'a NodeKey, &'a FsNode);
    type IntoIter = Iter<'a, NodeKey, FsNode>;
    
    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}


#[cfg(test)]
mod tests {
    use super::SortedNodes;
    use super::FsNode;
    
    #[test]
    fn test_sorting() {
        let n0 = FsNode::new_dir("A");
        let n2 = FsNode::new_dir("a");
        let n1 = FsNode::new_dir("_");
        let n3 = FsNode::new_file(None, "A", []);
        
        let mut nodes = SortedNodes::new();
        nodes.insert(n0.clone());
        nodes.insert(n3.clone());
        nodes.insert(n1.clone());
        nodes.insert(n2.clone());
        
        let mut iter = nodes.iter();
        
        assert_eq!(iter.next(), Some(&n0));
        assert_eq!(iter.next(), Some(&n1));
        assert_eq!(iter.next(), Some(&n2));
        assert_eq!(iter.next(), Some(&n3));
        assert_eq!(iter.next(), None)
    }
}