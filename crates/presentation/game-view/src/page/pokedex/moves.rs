use super::*;

pub(super) fn project(pokedex: &PokedexPageModel) -> Result<UiNode<PageIntent>, UiBuildError> {
    super::pokedex_moves_panel(pokedex)
}
