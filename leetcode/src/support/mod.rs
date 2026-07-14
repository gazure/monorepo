//! Shared data structures matching LeetCode's Rust definitions, plus builders and converters for
//! writing tests against linked-list and binary-tree problems.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

/// LeetCode's singly-linked list node definition.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ListNode {
    pub val: i32,
    pub next: Option<Box<ListNode>>,
}

impl ListNode {
    #[inline]
    pub fn new(val: i32) -> Self {
        ListNode { next: None, val }
    }
}

/// Build a linked list from a slice: `list_from(&[1, 2, 3])` is the list `1 -> 2 -> 3`.
pub fn list_from(vals: &[i32]) -> Option<Box<ListNode>> {
    let mut head = None;
    for &val in vals.iter().rev() {
        head = Some(Box::new(ListNode { val, next: head }));
    }
    head
}

/// Collect a linked list's values into a `Vec`, consuming the list.
pub fn list_to_vec(mut head: Option<Box<ListNode>>) -> Vec<i32> {
    let mut out = Vec::new();
    while let Some(node) = head {
        out.push(node.val);
        head = node.next;
    }
    out
}

/// LeetCode's binary tree node definition.
#[derive(Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub val: i32,
    pub left: Option<Rc<RefCell<TreeNode>>>,
    pub right: Option<Rc<RefCell<TreeNode>>>,
}

impl TreeNode {
    #[inline]
    pub fn new(val: i32) -> Self {
        TreeNode {
            val,
            left: None,
            right: None,
        }
    }
}

/// Build a binary tree from LeetCode's level-order representation, where `None` stands for the
/// `null` entries: `tree_from(&[Some(1), Some(2), Some(3), None, Some(4)])`.
pub fn tree_from(vals: &[Option<i32>]) -> Option<Rc<RefCell<TreeNode>>> {
    let mut iter = vals.iter().copied();
    let root_val = iter.next().flatten()?;
    let root = Rc::new(RefCell::new(TreeNode::new(root_val)));
    let mut queue = VecDeque::from([Rc::clone(&root)]);
    while let Some(node) = queue.pop_front() {
        let left = attach(iter.next(), &mut queue);
        let right = attach(iter.next(), &mut queue);
        let mut node = node.borrow_mut();
        node.left = left;
        node.right = right;
    }
    Some(root)
}

// The nesting is meaningful: outer None = level-order input exhausted, inner None = a `null` slot.
#[expect(clippy::option_option)]
fn attach(slot: Option<Option<i32>>, queue: &mut VecDeque<Rc<RefCell<TreeNode>>>) -> Option<Rc<RefCell<TreeNode>>> {
    let val = slot??;
    let child = Rc::new(RefCell::new(TreeNode::new(val)));
    queue.push_back(Rc::clone(&child));
    Some(child)
}

/// Convert a binary tree back to its level-order representation, trimming trailing `None`s the
/// way LeetCode renders trees.
pub fn tree_to_vec(root: &Option<Rc<RefCell<TreeNode>>>) -> Vec<Option<i32>> {
    let mut out = Vec::new();
    let mut queue = VecDeque::from([root.clone()]);
    while let Some(slot) = queue.pop_front() {
        if let Some(node) = slot {
            let node = node.borrow();
            out.push(Some(node.val));
            queue.push_back(node.left.clone());
            queue.push_back(node.right.clone());
        } else {
            out.push(None);
        }
    }
    while matches!(out.last(), Some(None)) {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_round_trip() {
        assert_eq!(list_to_vec(list_from(&[1, 2, 3])), vec![1, 2, 3]);
        assert_eq!(list_to_vec(list_from(&[])), Vec::<i32>::new());
        assert_eq!(list_from(&[]), None);
    }

    #[test]
    fn list_structure() {
        let list = list_from(&[7, 8]).unwrap();
        assert_eq!(list.val, 7);
        assert_eq!(list.next.as_ref().unwrap().val, 8);
        assert_eq!(list.next.unwrap().next, None);
    }

    #[test]
    fn tree_round_trip() {
        let repr = vec![Some(3), Some(9), Some(20), None, None, Some(15), Some(7)];
        assert_eq!(tree_to_vec(&tree_from(&repr)), repr);
    }

    #[test]
    fn tree_trims_trailing_nulls() {
        let tree = tree_from(&[Some(1), Some(2), None, None, None]);
        assert_eq!(tree_to_vec(&tree), vec![Some(1), Some(2)]);
    }

    #[test]
    fn tree_empty() {
        assert_eq!(tree_from(&[]), None);
        assert_eq!(tree_from(&[None]), None);
        assert_eq!(tree_to_vec(&None), Vec::<Option<i32>>::new());
    }

    #[test]
    fn tree_structure() {
        let root = tree_from(&[Some(1), None, Some(2)]).unwrap();
        let root = root.borrow();
        assert_eq!(root.val, 1);
        assert!(root.left.is_none());
        assert_eq!(root.right.as_ref().unwrap().borrow().val, 2);
    }
}
