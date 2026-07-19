use super::NodePath;

#[test]
fn path_prefix_observes_segment_boundaries() {
    let root = NodePath::parse("/").unwrap();
    let battle = NodePath::parse("/battle").unwrap();
    assert!(root.is_prefix_of(&battle));
    assert!(battle.is_prefix_of(&battle));
    assert!(battle.is_prefix_of(&NodePath::parse("/battle/turn").unwrap()));
    assert!(!battle.is_prefix_of(&NodePath::parse("/battles").unwrap()));
}
