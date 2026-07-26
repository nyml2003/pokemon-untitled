use std::collections::BTreeSet;

use super::{
    BagFilter, NationalDexNumber, PageDemoContext, PageDemoId, PageEffect, PageIntent, PageModel,
    PageState, PausePage, PausePageModel, PauseRoute, PlayerPage, PlayerRoute, demo_for,
    demo_named, page_demos, project_page,
};
use game_foundation::{CreatureId, GameIdError, ItemId, ShopId};
use game_session::ProductCommand;

#[test]
fn page_demo_catalog_covers_each_current_page_once() {
    let pages = page_demos()
        .iter()
        .map(|demo| demo.page())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        pages,
        BTreeSet::from([
            PlayerPage::World,
            PlayerPage::Pause,
            PlayerPage::Shop,
            PlayerPage::SaveConfirm,
        ])
    );
    let ids = page_demos()
        .iter()
        .map(|demo| demo.id())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), page_demos().len());
    assert!(demo_for(PageDemoId::new("shop-potion-preview")).is_some());
    assert!(demo_named("shop-potion-preview").is_some());
    assert!(demo_named("party-single-member").is_some());
    assert!(demo_named("bag-potion-list").is_some());
    assert!(demo_named("pokedex-seen-and-unseen").is_some());
    assert!(demo_named("trainer-card-starting-town").is_some());
    assert!(demo_named("not-a-page-demo").is_none());
}

#[test]
fn standard_demos_project_without_a_renderer() -> Result<(), Box<dyn std::error::Error>> {
    for demo in page_demos() {
        let model = demo.model()?;
        assert_eq!(model_page(&model), demo.page());
    }
    Ok(())
}

#[test]
fn pause_shop_and_save_reducers_only_emit_typed_requests() -> Result<(), Box<dyn std::error::Error>>
{
    let shop = id(ShopId::new("town-mart"))?;
    let potion = id(ItemId::new("potion"))?;
    let (state, effect) = PageState::world().transition(PageIntent::OpenPause)?;
    assert!(effect.is_none());
    let (state, effect) = state.transition(PageIntent::SelectPausePage(PausePage::Bag))?;
    assert!(effect.is_none());
    assert_eq!(
        state.route(),
        &PlayerRoute::Pause {
            route: PauseRoute::Bag {
                category: BagFilter::All,
                selected: None,
            }
        }
    );
    let (state, _) = state.transition(PageIntent::SelectBagItem(potion.clone()))?;
    assert_eq!(
        state.route(),
        &PlayerRoute::Pause {
            route: PauseRoute::Bag {
                category: BagFilter::All,
                selected: Some(potion.clone()),
            },
        }
    );
    let (state, _) = state.transition(PageIntent::Close)?;
    assert_eq!(
        state.route(),
        &PlayerRoute::Pause {
            route: PauseRoute::Menu,
        }
    );

    let (state, _) = PageState::world().transition(PageIntent::OpenShop { shop })?;
    let (state, _) = state.transition(PageIntent::SelectShopItem(potion.clone()))?;
    let (state, _) = state.transition(PageIntent::SetShopQuantity(2))?;
    let (_, effect) = state.transition(PageIntent::ConfirmShopPurchase)?;
    assert!(matches!(
        effect,
        Some(PageEffect::SubmitProduct(ProductCommand::BuyFromFront {
            item,
            quantity: 2,
        })) if item == potion
    ));

    let (state, _) = PageState::world().transition(PageIntent::OpenSaveConfirm)?;
    let (_, effect) = state.transition(PageIntent::ConfirmSave)?;
    assert!(matches!(effect, Some(PageEffect::RequestSave)));
    Ok(())
}

#[test]
fn gift_backed_pause_pages_read_product_facts() -> Result<(), Box<dyn std::error::Error>> {
    let party = demo_named("party-single-member")
        .ok_or("party demo is missing")?
        .model()?;
    let PageModel::Pause(PausePageModel::Party(party)) = party else {
        return Err("party demo did not project a party page".into());
    };
    assert_eq!(party.members.len(), 1);
    assert_eq!(party.members[0].species, "Treecko");

    let bag = demo_named("bag-potion-list")
        .ok_or("bag demo is missing")?
        .model()?;
    let PageModel::Pause(PausePageModel::Bag(bag)) = bag else {
        return Err("bag demo did not project a bag page".into());
    };
    assert_eq!(bag.entries.len(), 1);
    assert_eq!(bag.entries[0].item.as_str(), "potion");

    let pokedex = demo_named("pokedex-seen-and-unseen")
        .ok_or("pokedex demo is missing")?
        .model()?;
    let PageModel::Pause(PausePageModel::Pokedex(pokedex)) = pokedex else {
        return Err("pokedex demo did not project a pokedex page".into());
    };
    assert!(pokedex.selected.known);
    assert_eq!(pokedex.selected.number, NationalDexNumber::first());
    assert_eq!(pokedex.entries.len(), 386);
    assert_eq!(pokedex.known_count, 381);
    assert!(pokedex.entries[0].known);
    assert!(pokedex.entries[1].known);
    assert!(!pokedex.entries[3].known);
    assert!(pokedex.entries[19].known);
    assert!(pokedex.entries[20].known);
    assert!(pokedex.entries[251].known);
    assert_eq!(pokedex.selected.genus.as_deref(), Some("种子宝可梦"));
    assert_eq!(pokedex.selected.height_decimeters, Some(7));
    assert_eq!(pokedex.selected.weight_hectograms, Some(69));
    assert_eq!(pokedex.previous, None);
    Ok(())
}

#[test]
fn bag_filter_keeps_stable_entries_and_allows_empty_categories()
-> Result<(), Box<dyn std::error::Error>> {
    let context = PageDemoContext::gift_received()?;
    let route = PlayerRoute::Pause {
        route: PauseRoute::Bag {
            category: BagFilter::Category(game_foundation::ItemCategory::Key),
            selected: None,
        },
    };
    let PageModel::Pause(PausePageModel::Bag(key_page)) =
        project_page(context.content(), context.snapshot(), &route)?
    else {
        return Err("key filter did not project a bag page".into());
    };
    assert_eq!(
        key_page.category,
        BagFilter::Category(game_foundation::ItemCategory::Key)
    );
    assert!(key_page.entries.is_empty());

    let route = PlayerRoute::Pause {
        route: PauseRoute::Bag {
            category: BagFilter::Category(game_foundation::ItemCategory::Medicine),
            selected: None,
        },
    };
    let PageModel::Pause(PausePageModel::Bag(medicine_page)) =
        project_page(context.content(), context.snapshot(), &route)?
    else {
        return Err("medicine filter did not project a bag page".into());
    };
    assert_eq!(medicine_page.entries.len(), 1);
    assert_eq!(medicine_page.entries[0].item.as_str(), "potion");
    Ok(())
}

#[test]
fn pause_selection_intents_are_scoped_to_the_open_subpage() -> Result<(), Box<dyn std::error::Error>>
{
    let creature = id(CreatureId::new("starter-treecko-1"))?;
    let (state, _) = PageState::world().transition(PageIntent::OpenPause)?;
    let error = state
        .clone()
        .transition(PageIntent::SelectPartyMember(creature.clone()))
        .err()
        .ok_or("menu accepted a party selection")?;
    assert!(matches!(
        error,
        super::PageRouteError::IntentUnavailable { .. }
    ));
    let (state, _) = state.transition(PageIntent::SelectPausePage(PausePage::Party))?;
    let (state, _) = state.transition(PageIntent::SelectPartyMember(creature.clone()))?;
    assert_eq!(
        state.route(),
        &PlayerRoute::Pause {
            route: PauseRoute::Party {
                selected: Some(creature),
            },
        }
    );
    Ok(())
}

#[test]
fn shop_model_reads_current_snapshot_instead_of_route_copies()
-> Result<(), Box<dyn std::error::Error>> {
    let context = PageDemoContext::standard()?;
    let route = PlayerRoute::Shop {
        shop: id(ShopId::new("town-mart"))?,
        selected_item: Some(id(ItemId::new("potion"))?),
        quantity: 2,
    };
    let model = project_page(context.content(), context.snapshot(), &route)?;
    let PageModel::Shop(shop) = model else {
        return Err("shop route did not project a shop model".into());
    };
    let item = shop
        .selected_item
        .ok_or("shop demo did not keep its selected item")?;
    assert_eq!(item.unit_price.amount(), 30);
    assert_eq!(item.total_price.amount(), 60);
    assert!(item.affordable);
    assert_eq!(shop.money.amount(), 200);
    Ok(())
}

fn id<T>(value: Result<T, GameIdError>) -> Result<T, Box<dyn std::error::Error>> {
    value.map_err(|error| std::io::Error::other(format!("invalid fixture id: {error:?}")).into())
}

fn model_page(model: &PageModel) -> PlayerPage {
    match model {
        PageModel::World(_) => PlayerPage::World,
        PageModel::Pause(_) => PlayerPage::Pause,
        PageModel::Shop(_) => PlayerPage::Shop,
        PageModel::SaveConfirm(_) => PlayerPage::SaveConfirm,
    }
}
