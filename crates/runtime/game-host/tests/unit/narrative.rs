use super::load_narrative_scripts;

#[test]
fn checked_in_demo_scripts_compile_and_bind_every_npc() {
    let scripts = load_narrative_scripts().unwrap();
    assert_eq!(scripts.len(), 4);
    assert_eq!(
        scripts
            .iter()
            .map(|script| script.actor().unwrap().as_str())
            .collect::<Vec<_>>(),
        [
            "actor:forest-guide",
            "actor:forest-scout",
            "actor:forest-ranger",
            "actor:forest-collector",
        ]
    );
}
