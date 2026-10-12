//! [`NodePath`]: where an element sits under its window, as the index of
//! each child on the way down.
//!
//! The window reader (CAP-05) keeps every child up to its node budget in
//! order and only ever drops the *last* children of a subtree, so a path
//! found in a snapshot is the same path in the live tree as long as the
//! window has not changed. The driver walks it again before acting and
//! checks the element's role, so a window that changed in between fails
//! with [`ChatError::Moved`](crate::ChatError::Moved) instead of acting on
//! the wrong element.

use deino_speaker_tags::AxNode;

/// The child indices from a window down to one element.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodePath(Vec<usize>);

impl NodePath {
    /// The path through `indices`, from the window down.
    pub fn new(indices: Vec<usize>) -> Self {
        Self(indices)
    }

    /// The child indices, from the window down.
    pub fn indices(&self) -> &[usize] {
        &self.0
    }

    /// The element at this path under `window`, if there is one.
    pub fn resolve<'a>(&self, window: &'a AxNode) -> Option<&'a AxNode> {
        self.0
            .iter()
            .try_fold(window, |node, &index| node.children.get(index))
    }

    /// The path of the element's parent; `None` for the window itself.
    pub fn parent(&self) -> Option<Self> {
        let (_, parent) = self.0.split_last()?;
        Some(Self(parent.to_vec()))
    }

    /// The path of `index`-th child of this element.
    fn child(&self, index: usize) -> Self {
        let mut indices = self.0.clone();
        indices.push(index);
        Self(indices)
    }
}

/// The first element under `root` (itself included), depth first, that
/// `pick` accepts, with its path relative to `root`.
pub fn find_first(root: &AxNode, pick: &dyn Fn(&AxNode) -> bool) -> Option<NodePath> {
    let mut stack = vec![(NodePath::default(), root)];
    while let Some((path, node)) = stack.pop() {
        if pick(node) {
            return Some(path);
        }
        for (index, child) in node.children.iter().enumerate().rev() {
            stack.push((path.child(index), child));
        }
    }
    None
}

/// Joins `below` (relative to the element at `base`) onto `base`.
pub fn join(base: &NodePath, below: &NodePath) -> NodePath {
    let mut indices = base.0.clone();
    indices.extend_from_slice(&below.0);
    NodePath(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: &str, children: Vec<AxNode>) -> AxNode {
        AxNode {
            role: role.into(),
            children,
            ..AxNode::default()
        }
    }

    #[test]
    fn paths_find_and_resolve_the_same_element() {
        let window = node(
            "AXWindow",
            vec![
                node("AXGroup", vec![node("AXButton", vec![])]),
                node("AXGroup", vec![node("AXTextArea", vec![])]),
            ],
        );

        let path = find_first(&window, &|node| node.is("AXTextArea")).unwrap();

        assert_eq!(path.indices(), [1, 0]);
        assert_eq!(path.resolve(&window).unwrap().role, "AXTextArea");
        assert_eq!(path.parent().unwrap().indices(), [1]);
        assert_eq!(join(&NodePath::new(vec![1]), &NodePath::new(vec![0])), path);
        assert!(NodePath::default().parent().is_none());
        assert!(NodePath::new(vec![5]).resolve(&window).is_none());
    }

    #[test]
    fn the_first_match_is_depth_first_in_order() {
        let window = node(
            "AXWindow",
            vec![
                node("AXGroup", vec![node("AXButton", vec![])]),
                node("AXButton", vec![]),
            ],
        );

        let path = find_first(&window, &|node| node.is("AXButton")).unwrap();

        assert_eq!(path.indices(), [0, 0]);
    }
}
