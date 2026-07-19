use super::*;

#[test]
fn resource_ids_are_typed_and_validate_every_error() {
    assert_eq!(
        ScriptId::new("script:opening").unwrap().as_str(),
        "script:opening"
    );
    assert_eq!(TextId::new("text:hello").unwrap().as_str(), "text:hello");
    assert_eq!(EventId::new("event:clear").unwrap().as_str(), "event:clear");
    assert_eq!(ActorId::new("actor:guide").unwrap().as_str(), "actor:guide");
    for value in [
        "script",
        "wrong:name",
        "script:",
        "script:9bad",
        "script:bad/name",
    ] {
        assert!(ScriptId::new(value).is_err());
    }
    assert_eq!(
        ResourceIdError::MissingSeparator.to_string(),
        "resource ID must contain ':'"
    );
    assert_eq!(
        ResourceIdError::WrongPrefix {
            expected: "script",
            actual: "event".into(),
        }
        .to_string(),
        "expected script: resource, got event:"
    );
    assert_eq!(
        ResourceIdError::InvalidName.to_string(),
        "resource ID name is invalid"
    );
}

#[test]
fn programs_validate_entries_targets_and_accessors() {
    let id = ScriptId::new("script:opening").unwrap();
    let entry = ContinuationId::new(0);
    let next = ContinuationId::new(1);
    let mut valid = BTreeMap::new();
    valid.insert(
        entry,
        CpsNode::Say {
            text: TextId::new("text:hello").unwrap(),
            next,
        },
    );
    valid.insert(
        next,
        CpsNode::Wait {
            event: EventId::new("event:clear").unwrap(),
            resume: ContinuationId::new(2),
        },
    );
    valid.insert(ContinuationId::new(2), CpsNode::End);
    let program = ScriptProgram::new(id, entry, valid).unwrap();
    assert_eq!(program.id().as_str(), "script:opening");
    assert_eq!(program.actor(), None);
    assert_eq!(program.entry().value(), 0);
    assert!(matches!(
        program.continuation(next),
        Some(CpsNode::Wait { .. })
    ));
    assert_eq!(program.continuations().len(), 3);

    let missing_entry = ScriptProgram::new(
        ScriptId::new("script:missing").unwrap(),
        ContinuationId::new(9),
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(missing_entry.to_string(), "missing entry continuation 9");

    let mut missing_target = BTreeMap::new();
    missing_target.insert(
        entry,
        CpsNode::Say {
            text: TextId::new("text:hello").unwrap(),
            next,
        },
    );
    let error = ScriptProgram::new(
        ScriptId::new("script:target").unwrap(),
        entry,
        missing_target,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "continuation 0 targets missing continuation 1"
    );
}

#[test]
fn actor_bound_programs_expose_movement_and_facing_nodes() {
    let entry = ContinuationId::new(0);
    let face = ContinuationId::new(1);
    let mut continuations = BTreeMap::new();
    continuations.insert(
        entry,
        CpsNode::Move {
            direction: ScriptDirection::Right,
            next: face,
        },
    );
    continuations.insert(
        face,
        CpsNode::Face {
            direction: ScriptDirection::Up,
            next: ContinuationId::new(2),
        },
    );
    continuations.insert(ContinuationId::new(2), CpsNode::End);
    let program = ScriptProgram::with_actor(
        ScriptId::new("script:guide").unwrap(),
        Some(ActorId::new("actor:guide").unwrap()),
        entry,
        continuations,
    )
    .unwrap();

    assert_eq!(program.actor().unwrap().as_str(), "actor:guide");
    assert_eq!(
        ScriptDirection::from_keyword("left"),
        Some(ScriptDirection::Left)
    );
    assert_eq!(ScriptDirection::from_keyword("unknown"), None);
    assert!(matches!(
        program.continuation(entry),
        Some(CpsNode::Move {
            direction: ScriptDirection::Right,
            ..
        })
    ));
    assert!(matches!(
        program.continuation(face),
        Some(CpsNode::Face {
            direction: ScriptDirection::Up,
            ..
        })
    ));
}
