use super::*;

pub(super) fn project(
    pokedex: &PokedexPageModel,
    interactive: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    super::pokedex_stats_page(pokedex, interactive)
}
