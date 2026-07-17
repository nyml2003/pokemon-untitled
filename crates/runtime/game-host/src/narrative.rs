use std::io;

use narrative_compiler::Compiler;
use narrative_cps::ScriptProgram;
use narrative_token::SliceByteStream;

const DEMO_SOURCES: [(&str, &str); 4] = [
    (
        "forest-guide.narrative",
        include_str!("../../../../narrative/demo/forest-guide.narrative"),
    ),
    (
        "forest-scout.narrative",
        include_str!("../../../../narrative/demo/forest-scout.narrative"),
    ),
    (
        "forest-ranger.narrative",
        include_str!("../../../../narrative/demo/forest-ranger.narrative"),
    ),
    (
        "forest-collector.narrative",
        include_str!("../../../../narrative/demo/forest-collector.narrative"),
    ),
];

pub fn load_narrative_scripts() -> Result<Vec<ScriptProgram>, io::Error> {
    DEMO_SOURCES
        .into_iter()
        .map(|(name, source)| {
            let outcome = Compiler::compile(SliceByteStream::new(source.as_bytes()));
            outcome.program().cloned().ok_or_else(|| {
                let messages = outcome
                    .diagnostics()
                    .iter()
                    .map(|diagnostic| diagnostic.message())
                    .collect::<Vec<_>>()
                    .join(", ");
                io::Error::other(format!("compile narrative/{name}: {messages}"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
}
