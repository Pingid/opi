use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Tree<T> {
    Item(T),
    Map(IndexMap<String, Option<Tree<T>>>),
}

// // impl<K: Eq + Hash, T> Tree<K, T> {
// //     pub fn leaf(value: T) -> Self {
// //         Tree::Leaf(value)
// //     }

// //     pub fn node(children: impl IntoIterator<Item = (K, Tree<K, T>)>) -> Self {
// //         Tree::Node(children.into_iter().collect())
// //     }

// //     pub fn is_leaf(&self) -> bool {
// //         matches!(self, Tree::Leaf(_))
// //     }

// //     pub fn is_node(&self) -> bool {
// //         matches!(self, Tree::Node(_))
// //     }

// //     pub fn is_empty(&self) -> bool {
// //         matches!(self, Tree::Node(children) if children.is_empty())
// //     }

// //     /// Returns a depth-first pre-order iterator over the tree's (key, node) pairs.
// //     pub fn iter(&self) -> TreeIter<'_, K, T> {
// //         TreeIter::new(self)
// //     }
// // }

// // pub struct TreeIter<'a, K: Eq + Hash, T> {
// //     stack: Vec<indexmap::map::Iter<'a, K, Tree<K, T>>>,
// //     root_leaf: Option<&'a Tree<K, T>>,
// // }

// // impl<'a, K: Eq + Hash, T> TreeIter<'a, K, T> {
// //     pub fn new(root: &'a Tree<K, T>) -> Self {
// //         match root {
// //             Tree::Node(children) => Self {
// //                 stack: vec![children.iter()],
// //                 root_leaf: None,
// //             },
// //             leaf @ Tree::Leaf(_) => Self {
// //                 stack: Vec::new(),
// //                 root_leaf: Some(leaf),
// //             },
// //         }
// //     }
// // }

// // impl<'a, K: Eq + Hash, T> Iterator for TreeIter<'a, K, T> {
// //     type Item = (&'a K, &'a Tree<K, T>);

// //     fn next(&mut self) -> Option<Self::Item> {
// //         if let Some(leaf) = self.root_leaf.take() {
// //             return Some(("", leaf));
// //         }
// //         while let Some(top_iter) = self.stack.last_mut() {
// //             if let Some((key, child)) = top_iter.next() {
// //                 if let Tree::Node(grand_children) = child {
// //                     self.stack.push(grand_children.iter());
// //                 }
// //                 return Some((key.as_str(), &child));
// //             } else {
// //                 self.stack.pop();
// //             }
// //         }

// //         None
// //     }
// // }

// // impl<'a, K, T> IntoIterator for &'a Tree<K, T> {
// //     type Item = (&'a K, &'a Tree<K, T>);
// //     type IntoIter = TreeIter<'a, K, T>;

// //     fn into_iter(self) -> Self::IntoIter {
// //         self.iter()
// //     }
// // }
