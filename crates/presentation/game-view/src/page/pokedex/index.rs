use super::*;

pub(super) fn project(pokedex: &PokedexPageModel) -> Result<UiNode<PageIntent>, UiBuildError> {
    super::pokedex_index_page(pokedex)
}
