//! 渲染无关的玩家页面路由、模型与 demo 目录。
//!
//! 此 crate 只从产品快照投影页面模型，并发出类型化页面意图或产品请求。
//! 它不读取输入设备、不创建窗口，也不生成渲染命令。

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use game_data::{
    CurrentDataSet, DamageClass, DataLoadError, GEN3_FIRST_DEX, GEN3_LAST_DEX, MoveLearnMethod,
    PokedexData, PokedexLoadError,
};
use game_foundation::{
    ContentError, CreatureId, CreatureTemplateId, Direction, GameIdError, ItemCategory, ItemId,
    Money, ShopId, ThinSliceContent,
};
use game_session::{ProductCommand, ProductError, ProductSession, ProductSnapshot};

/// 代码和开发入口共用的稳定页面 demo 标识。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageDemoId(&'static str);

impl PageDemoId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerPage {
    World,
    Pause,
    Shop,
    SaveConfirm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PausePage {
    Menu,
    Party,
    Bag,
    Pokedex,
    TrainerCard,
}

/// 背包页面使用的稳定分类值。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BagFilter {
    All,
    Category(ItemCategory),
}

impl BagFilter {
    pub fn matches(self, category: ItemCategory) -> bool {
        match self {
            Self::All => true,
            Self::Category(expected) => expected == category,
        }
    }
}

/// 已验证位于 Gen3 全国图鉴范围内的号码。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NationalDexNumber(u16);

impl NationalDexNumber {
    pub fn new(value: u16) -> Result<Self, NationalDexError> {
        if (GEN3_FIRST_DEX..=GEN3_LAST_DEX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(NationalDexError::OutOfRange(value))
        }
    }

    pub const fn first() -> Self {
        Self(GEN3_FIRST_DEX)
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub fn previous(self) -> Option<Self> {
        self.0
            .checked_sub(1)
            .filter(|value| *value >= GEN3_FIRST_DEX)
            .map(Self)
    }

    pub fn next(self) -> Option<Self> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= GEN3_LAST_DEX)
            .map(Self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NationalDexError {
    OutOfRange(u16),
}

impl fmt::Display for NationalDexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange(value) => {
                write!(formatter, "national dex number {value} is out of range")
            }
        }
    }
}

impl Error for NationalDexError {}

/// 暂停页内部导航和稳定选择值。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PauseRoute {
    Menu,
    Party {
        selected: Option<CreatureId>,
    },
    Bag {
        category: BagFilter,
        selected: Option<ItemId>,
    },
    Pokedex {
        selected: NationalDexNumber,
        stats_view: PokedexStatsView,
        selected_move: usize,
    },
    TrainerCard,
}

impl PauseRoute {
    pub const fn page(&self) -> PausePage {
        match self {
            Self::Menu => PausePage::Menu,
            Self::Party { .. } => PausePage::Party,
            Self::Bag { .. } => PausePage::Bag,
            Self::Pokedex { .. } => PausePage::Pokedex,
            Self::TrainerCard => PausePage::TrainerCard,
        }
    }

    fn for_page(page: PausePage) -> Self {
        match page {
            PausePage::Menu => Self::Menu,
            PausePage::Party => Self::Party { selected: None },
            PausePage::Bag => Self::Bag {
                category: BagFilter::All,
                selected: None,
            },
            PausePage::Pokedex => Self::Pokedex {
                selected: NationalDexNumber::first(),
                stats_view: PokedexStatsView::Bars,
                selected_move: 0,
            },
            PausePage::TrainerCard => Self::TrainerCard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerRoute {
    World,
    Pause {
        route: PauseRoute,
    },
    Shop {
        shop: ShopId,
        selected_item: Option<ItemId>,
        quantity: u16,
    },
    SaveConfirm,
}

impl PlayerRoute {
    pub const fn page(&self) -> PlayerPage {
        match self {
            Self::World => PlayerPage::World,
            Self::Pause { .. } => PlayerPage::Pause,
            Self::Shop { .. } => PlayerPage::Shop,
            Self::SaveConfirm => PlayerPage::SaveConfirm,
        }
    }
}

/// 设备适配层翻译后的页面意图。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageIntent {
    OpenPause,
    SelectPausePage(PausePage),
    SelectPartyMember(CreatureId),
    SelectBagCategory(BagFilter),
    SelectBagItem(ItemId),
    SelectPokedexEntry(NationalDexNumber),
    TogglePokedexStatsView,
    SelectPokedexMove(usize),
    OpenShop { shop: ShopId },
    SelectShopItem(ItemId),
    SetShopQuantity(u16),
    ConfirmShopPurchase,
    OpenSaveConfirm,
    ConfirmSave,
    Close,
}

/// 页面请求由应用层执行，页面 reducer 不直接修改产品状态。
pub enum PageEffect {
    SubmitProduct(ProductCommand),
    RequestSave,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRouteError {
    IntentUnavailable {
        page: PlayerPage,
        intent: PageIntent,
    },
    SelectionRequired {
        page: PlayerPage,
    },
    ZeroQuantity,
}

impl fmt::Display for PageRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntentUnavailable { page, intent } => {
                write!(formatter, "intent {intent:?} is unavailable on {page:?}")
            }
            Self::SelectionRequired { page } => write!(formatter, "{page:?} requires a selection"),
            Self::ZeroQuantity => write!(formatter, "shop quantity must be greater than zero"),
        }
    }
}

impl Error for PageRouteError {}

/// 页面局部状态只保存路由与选择，不缓存游戏事实。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageState {
    route: PlayerRoute,
}

impl PageState {
    pub const fn world() -> Self {
        Self {
            route: PlayerRoute::World,
        }
    }

    pub fn from_route(route: PlayerRoute) -> Self {
        Self { route }
    }

    pub const fn route(&self) -> &PlayerRoute {
        &self.route
    }

    /// 应用局部意图，必要时返回交由应用层执行的请求。
    pub fn transition(
        mut self,
        intent: PageIntent,
    ) -> Result<(Self, Option<PageEffect>), PageRouteError> {
        let page = self.route.page();
        match (&mut self.route, intent) {
            (PlayerRoute::World, PageIntent::OpenPause) => {
                self.route = PlayerRoute::Pause {
                    route: PauseRoute::Menu,
                };
                Ok((self, None))
            }
            (PlayerRoute::World, PageIntent::OpenShop { shop }) => {
                self.route = PlayerRoute::Shop {
                    shop,
                    selected_item: None,
                    quantity: 1,
                };
                Ok((self, None))
            }
            (PlayerRoute::World, PageIntent::OpenSaveConfirm) => {
                self.route = PlayerRoute::SaveConfirm;
                Ok((self, None))
            }
            (PlayerRoute::Pause { route }, PageIntent::SelectPausePage(next)) => {
                *route = PauseRoute::for_page(next);
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Party { selected },
                },
                PageIntent::SelectPartyMember(next),
            ) => {
                *selected = Some(next);
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Bag { category, .. },
                },
                PageIntent::SelectBagCategory(next),
            ) => {
                *category = next;
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Bag { selected, .. },
                },
                PageIntent::SelectBagItem(next),
            ) => {
                *selected = Some(next);
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route:
                        PauseRoute::Pokedex {
                            selected,
                            selected_move,
                            ..
                        },
                },
                PageIntent::SelectPokedexEntry(next),
            ) => {
                *selected = next;
                *selected_move = 0;
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Pokedex { stats_view, .. },
                },
                PageIntent::TogglePokedexStatsView,
            ) => {
                *stats_view = stats_view.toggled();
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Pokedex { selected_move, .. },
                },
                PageIntent::SelectPokedexMove(next),
            ) => {
                *selected_move = next;
                Ok((self, None))
            }
            (
                PlayerRoute::Pause {
                    route: PauseRoute::Menu,
                },
                PageIntent::Close,
            ) => {
                self.route = PlayerRoute::World;
                Ok((self, None))
            }
            (PlayerRoute::Pause { .. }, PageIntent::Close) => {
                self.route = PlayerRoute::Pause {
                    route: PauseRoute::Menu,
                };
                Ok((self, None))
            }
            (PlayerRoute::Shop { selected_item, .. }, PageIntent::SelectShopItem(item)) => {
                *selected_item = Some(item);
                Ok((self, None))
            }
            (PlayerRoute::Shop { quantity, .. }, PageIntent::SetShopQuantity(next)) => {
                if next == 0 {
                    return Err(PageRouteError::ZeroQuantity);
                }
                *quantity = next;
                Ok((self, None))
            }
            (
                PlayerRoute::Shop {
                    selected_item: Some(item),
                    quantity,
                    ..
                },
                PageIntent::ConfirmShopPurchase,
            ) => {
                let effect = PageEffect::SubmitProduct(ProductCommand::BuyFromFront {
                    item: item.clone(),
                    quantity: *quantity,
                });
                Ok((self, Some(effect)))
            }
            (PlayerRoute::Shop { .. }, PageIntent::ConfirmShopPurchase) => {
                Err(PageRouteError::SelectionRequired { page })
            }
            (PlayerRoute::Shop { .. }, PageIntent::Close) => {
                self.route = PlayerRoute::World;
                Ok((self, None))
            }
            (PlayerRoute::SaveConfirm, PageIntent::ConfirmSave) => {
                Ok((self, Some(PageEffect::RequestSave)))
            }
            (PlayerRoute::SaveConfirm, PageIntent::Close) => {
                self.route = PlayerRoute::World;
                Ok((self, None))
            }
            (_, intent) => Err(PageRouteError::IntentUnavailable { page, intent }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageModel {
    World(WorldPageModel),
    Pause(PausePageModel),
    Shop(ShopPageModel),
    SaveConfirm(SaveConfirmPageModel),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldPageModel {
    pub location: String,
    pub party_count: usize,
    pub money: Money,
    pub save_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PausePageModel {
    Menu,
    Party(PartyPageModel),
    Bag(BagPageModel),
    Pokedex(PokedexPageModel),
    TrainerCard(TrainerCardPageModel),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyPageModel {
    pub selected: Option<CreatureId>,
    pub members: Vec<PartyMemberModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyMemberModel {
    pub id: CreatureId,
    pub species: String,
    pub current_hp: u16,
    pub max_hp: u16,
    pub current_pp: u8,
    pub max_pp: u8,
    pub experience: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BagPageModel {
    pub category: BagFilter,
    pub selected: Option<ItemId>,
    pub entries: Vec<BagItemModel>,
    pub money: Money,
    pub slots_used: usize,
    pub capacity: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BagItemModel {
    pub item: ItemId,
    pub category: ItemCategory,
    pub quantity: u16,
    pub stack_limit: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PokedexPageModel {
    pub selected: PokedexEntryModel,
    pub entries: Vec<PokedexEntryModel>,
    pub stats_view: PokedexStatsView,
    pub selected_move: usize,
    pub moves: Vec<PokedexMoveModel>,
    pub previous: Option<NationalDexNumber>,
    pub next: Option<NationalDexNumber>,
    pub known_count: usize,
    pub total_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PokedexEntryModel {
    pub number: NationalDexNumber,
    pub name: Option<String>,
    pub stats: Option<PokedexStatsModel>,
    pub types: Vec<String>,
    pub known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PokedexMoveModel {
    pub name: String,
    pub move_type: String,
    pub category: PokedexMoveCategory,
    pub power: Option<u16>,
    pub accuracy: Option<u8>,
    pub pp: Option<u8>,
    pub method: PokedexMoveLearnMethod,
    pub level: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PokedexMoveCategory {
    Physical,
    Special,
    Status,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PokedexMoveLearnMethod {
    LevelUp,
    Egg,
    Tutor,
    Machine,
    Other(String),
}

/// 图鉴横向深入轨道中的页面层级。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PokedexSection {
    #[default]
    Browse,
    Profile,
    Moves,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PokedexStatsModel {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PokedexStatsView {
    Bars,
    Hexagon,
}

impl PokedexStatsView {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Bars => Self::Hexagon,
            Self::Hexagon => Self::Bars,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PokedexDisplayMode {
    ProductFacts,
    DemoAllOpen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainerCardPageModel {
    pub location: String,
    pub money: Money,
    pub party_count: usize,
    pub defeated_trainers: usize,
    pub total_trainers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopPageModel {
    pub shop: ShopId,
    pub selected_item: Option<ShopItemModel>,
    pub money: Money,
    pub inventory_slots_used: usize,
    pub inventory_capacity: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopItemModel {
    pub item: ItemId,
    pub quantity: u16,
    pub owned_quantity: u16,
    pub unit_price: Money,
    pub total_price: Money,
    pub affordable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveConfirmPageModel {
    pub available: bool,
    pub unavailable_reason: Option<SaveUnavailableReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveUnavailableReason {
    BattleActive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageModelError {
    CreatureTemplateMissing(CreatureTemplateId),
    PartySelectionMissing(CreatureId),
    ItemMissing(ItemId),
    BagSelectionMissing(ItemId),
    Data(DataLoadError),
    Pokedex(PokedexLoadError),
    PokedexEntryMissing(NationalDexNumber),
    ShopMissing(ShopId),
    ShopListingMissing {
        shop: ShopId,
        item: ItemId,
    },
    PriceOverflow {
        shop: ShopId,
        item: ItemId,
        quantity: u16,
    },
}

impl fmt::Display for PageModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreatureTemplateMissing(template) => {
                write!(
                    formatter,
                    "creature template {} is missing",
                    template.as_str()
                )
            }
            Self::PartySelectionMissing(creature) => {
                write!(formatter, "party creature {} is missing", creature.as_str())
            }
            Self::ItemMissing(item) => write!(formatter, "item {} is missing", item.as_str()),
            Self::BagSelectionMissing(item) => {
                write!(formatter, "bag item {} is not carried", item.as_str())
            }
            Self::Data(error) => write!(formatter, "game data unavailable: {error}"),
            Self::Pokedex(error) => write!(formatter, "pokedex data unavailable: {error}"),
            Self::PokedexEntryMissing(number) => {
                write!(formatter, "pokedex entry {} is missing", number.value())
            }
            Self::ShopMissing(shop) => write!(formatter, "shop {} is missing", shop.as_str()),
            Self::ShopListingMissing { shop, item } => write!(
                formatter,
                "shop {} does not list item {}",
                shop.as_str(),
                item.as_str()
            ),
            Self::PriceOverflow {
                shop,
                item,
                quantity,
            } => write!(
                formatter,
                "price overflow for {} x{} in shop {}",
                item.as_str(),
                quantity,
                shop.as_str()
            ),
        }
    }
}

impl Error for PageModelError {}

/// 从权威内容和产品快照重建当前页面模型。
pub fn project_page(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    route: &PlayerRoute,
) -> Result<PageModel, PageModelError> {
    project_page_with_pokedex_mode(content, snapshot, route, PokedexDisplayMode::ProductFacts)
}

/// 使用 demo 上下文投影页面，允许 demo 保留独立的图鉴展示状态。
pub fn project_demo_page(
    context: &PageDemoContext,
    route: &PlayerRoute,
) -> Result<PageModel, PageModelError> {
    project_page_with_pokedex_mode(
        &context.content,
        &context.snapshot,
        route,
        context.pokedex_display_mode,
    )
}

fn project_page_with_pokedex_mode(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    route: &PlayerRoute,
    pokedex_display_mode: PokedexDisplayMode,
) -> Result<PageModel, PageModelError> {
    let state = snapshot.state();
    match route {
        PlayerRoute::World => Ok(PageModel::World(WorldPageModel {
            location: state.map().as_str().to_owned(),
            party_count: state.party().len(),
            money: state.money(),
            save_available: snapshot.save_available(),
        })),
        PlayerRoute::Pause { route } => {
            project_pause_page(content, snapshot, route, pokedex_display_mode)
        }
        PlayerRoute::Shop {
            shop,
            selected_item,
            quantity,
        } => {
            let definition = content
                .shop(shop)
                .ok_or_else(|| PageModelError::ShopMissing(shop.clone()))?;
            let selected_item = selected_item
                .as_ref()
                .map(|item| shop_item_model(definition, shop, item, *quantity, snapshot))
                .transpose()?;
            Ok(PageModel::Shop(ShopPageModel {
                shop: shop.clone(),
                selected_item,
                money: state.money(),
                inventory_slots_used: state.inventory().entries().len(),
                inventory_capacity: state.inventory().capacity(),
            }))
        }
        PlayerRoute::SaveConfirm => Ok(PageModel::SaveConfirm(SaveConfirmPageModel {
            available: snapshot.save_available(),
            unavailable_reason: (!snapshot.save_available())
                .then_some(SaveUnavailableReason::BattleActive),
        })),
    }
}

fn project_pause_page(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    route: &PauseRoute,
    pokedex_display_mode: PokedexDisplayMode,
) -> Result<PageModel, PageModelError> {
    let state = snapshot.state();
    let pause = match route {
        PauseRoute::Menu => PausePageModel::Menu,
        PauseRoute::Party { selected } => {
            PausePageModel::Party(party_page_model(content, snapshot, selected)?)
        }
        PauseRoute::Bag { category, selected } => {
            PausePageModel::Bag(bag_page_model(content, snapshot, *category, selected)?)
        }
        PauseRoute::Pokedex {
            selected,
            stats_view,
            selected_move,
        } => PausePageModel::Pokedex(pokedex_page_model(
            content,
            snapshot,
            *selected,
            *stats_view,
            *selected_move,
            pokedex_display_mode,
        )?),
        PauseRoute::TrainerCard => PausePageModel::TrainerCard(TrainerCardPageModel {
            location: state.map().as_str().to_owned(),
            money: state.money(),
            party_count: state.party().len(),
            defeated_trainers: state.defeated_trainers().len(),
            total_trainers: content.trainers().count(),
        }),
    };
    Ok(PageModel::Pause(pause))
}

fn party_page_model(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    selected: &Option<CreatureId>,
) -> Result<PartyPageModel, PageModelError> {
    let members = snapshot
        .state()
        .party()
        .iter()
        .map(|creature| {
            let template = content.creature(creature.template()).ok_or_else(|| {
                PageModelError::CreatureTemplateMissing(creature.template().clone())
            })?;
            Ok(PartyMemberModel {
                id: creature.id().clone(),
                species: template.species().to_owned(),
                current_hp: creature.hp(),
                max_hp: template.max_hp(),
                current_pp: creature.pp(),
                max_pp: template.max_pp(),
                experience: creature.experience(),
            })
        })
        .collect::<Result<Vec<_>, PageModelError>>()?;
    if let Some(selected) = selected
        && !members.iter().any(|member| &member.id == selected)
    {
        return Err(PageModelError::PartySelectionMissing(selected.clone()));
    }
    Ok(PartyPageModel {
        selected: selected.clone(),
        members,
    })
}

fn bag_page_model(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    category: BagFilter,
    selected: &Option<ItemId>,
) -> Result<BagPageModel, PageModelError> {
    let state = snapshot.state();
    let all_entries = state
        .inventory()
        .entries()
        .iter()
        .map(|(item, quantity)| {
            let definition = content
                .item(item)
                .ok_or_else(|| PageModelError::ItemMissing(item.clone()))?;
            Ok(BagItemModel {
                item: item.clone(),
                category: definition.category(),
                quantity: *quantity,
                stack_limit: definition.stack_limit(),
            })
        })
        .collect::<Result<Vec<_>, PageModelError>>()?;
    if let Some(selected) = selected
        && !all_entries.iter().any(|entry| &entry.item == selected)
    {
        return Err(PageModelError::BagSelectionMissing(selected.clone()));
    }
    let entries = all_entries
        .into_iter()
        .filter(|entry| category.matches(entry.category))
        .collect::<Vec<_>>();
    Ok(BagPageModel {
        category,
        selected: selected
            .clone()
            .filter(|selected| entries.iter().any(|entry| &entry.item == selected)),
        entries,
        money: state.money(),
        slots_used: state.inventory().entries().len(),
        capacity: state.inventory().capacity(),
    })
}

fn pokedex_page_model(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
    selected: NationalDexNumber,
    stats_view: PokedexStatsView,
    selected_move: usize,
    display_mode: PokedexDisplayMode,
) -> Result<PokedexPageModel, PageModelError> {
    let pokedex = PokedexData::embedded_gen3_shared().map_err(PageModelError::Pokedex)?;
    let data = CurrentDataSet::embedded_shared().map_err(PageModelError::Data)?;
    let known_species = party_species(content, snapshot)?;
    let entries = pokedex
        .entries()
        .iter()
        .map(|entry| {
            let product_known = known_species.contains(entry.english_name.as_str());
            let known = match display_mode {
                PokedexDisplayMode::ProductFacts => product_known,
                PokedexDisplayMode::DemoAllOpen => demo_pokedex_known(entry.national_dex),
            };
            let number = NationalDexNumber::new(entry.national_dex)
                .map_err(|_| PageModelError::PokedexEntryMissing(selected))?;
            Ok(PokedexEntryModel {
                number,
                name: known.then(|| entry.localized_name.clone()),
                stats: known.then_some(PokedexStatsModel {
                    hp: entry.base_stats.hp,
                    attack: entry.base_stats.attack,
                    defense: entry.base_stats.defense,
                    special_attack: entry.base_stats.special_attack,
                    special_defense: entry.base_stats.special_defense,
                    speed: entry.base_stats.speed,
                }),
                types: if known {
                    entry
                        .types
                        .iter()
                        .map(|entry_type| entry_type.name.clone())
                        .collect()
                } else {
                    Vec::new()
                },
                known,
            })
        })
        .collect::<Result<Vec<_>, PageModelError>>()?;
    let selected_entry = entries
        .iter()
        .find(|entry| entry.number == selected)
        .cloned()
        .ok_or(PageModelError::PokedexEntryMissing(selected))?;
    let selected_data_entry = pokedex
        .entries()
        .iter()
        .find(|entry| entry.national_dex == selected.value())
        .ok_or(PageModelError::PokedexEntryMissing(selected))?;
    let moves = if selected_entry.known {
        pokedex_moves(data.as_ref(), selected_data_entry.form_id)
    } else {
        Vec::new()
    };
    let known_count = entries.iter().filter(|entry| entry.known).count();
    Ok(PokedexPageModel {
        selected: selected_entry,
        entries,
        stats_view,
        selected_move: selected_move.min(moves.len().saturating_sub(1)),
        moves,
        previous: selected.previous(),
        next: selected.next(),
        known_count,
        total_count: pokedex.entries().len(),
    })
}

fn pokedex_moves(
    data: &CurrentDataSet,
    form_id: game_data::PokemonFormId,
) -> Vec<PokedexMoveModel> {
    let Some(learnset) = data.learnset(form_id) else {
        return Vec::new();
    };
    learnset
        .iter()
        .filter_map(|learnset_entry| {
            let record = data.move_by_id(learnset_entry.move_id)?;
            let move_type = data.type_by_id(record.move_type)?;
            Some(PokedexMoveModel {
                name: record.display_name.localized.clone(),
                move_type: move_type.display_name.localized.clone(),
                category: match record.damage_class {
                    DamageClass::Physical => PokedexMoveCategory::Physical,
                    DamageClass::Special => PokedexMoveCategory::Special,
                    DamageClass::Status => PokedexMoveCategory::Status,
                },
                power: record.power,
                accuracy: record.accuracy,
                pp: record.pp,
                method: match &learnset_entry.method {
                    MoveLearnMethod::LevelUp => PokedexMoveLearnMethod::LevelUp,
                    MoveLearnMethod::Egg => PokedexMoveLearnMethod::Egg,
                    MoveLearnMethod::Tutor => PokedexMoveLearnMethod::Tutor,
                    MoveLearnMethod::Machine => PokedexMoveLearnMethod::Machine,
                    MoveLearnMethod::Other(value) => PokedexMoveLearnMethod::Other(value.clone()),
                },
                level: learnset_entry.level,
            })
        })
        .collect()
}

fn demo_pokedex_known(number: u16) -> bool {
    match number {
        4 | 7 | 11 | 15 | 19 => false,
        2..=20 => true,
        _ => true,
    }
}

/// 队伍拥有事实是目前唯一可验证的图鉴已知来源，不伪造独立图鉴进度。
fn party_species(
    content: &ThinSliceContent,
    snapshot: &ProductSnapshot,
) -> Result<BTreeSet<String>, PageModelError> {
    snapshot
        .state()
        .party()
        .iter()
        .map(|creature| {
            content
                .creature(creature.template())
                .map(|template| template.species().to_owned())
                .ok_or_else(|| PageModelError::CreatureTemplateMissing(creature.template().clone()))
        })
        .collect()
}

fn shop_item_model(
    definition: &game_foundation::ShopDefinition,
    shop: &ShopId,
    item: &ItemId,
    quantity: u16,
    snapshot: &ProductSnapshot,
) -> Result<ShopItemModel, PageModelError> {
    let listing = definition
        .listing(item)
        .ok_or_else(|| PageModelError::ShopListingMissing {
            shop: shop.clone(),
            item: item.clone(),
        })?;
    let total_price = listing
        .total_price(quantity)
        .map_err(|_| PageModelError::PriceOverflow {
            shop: shop.clone(),
            item: item.clone(),
            quantity,
        })?;
    let state = snapshot.state();
    Ok(ShopItemModel {
        item: item.clone(),
        quantity,
        owned_quantity: state.inventory().quantity(item),
        unit_price: listing.unit_price(),
        total_price,
        affordable: state.money().amount() >= total_price.amount(),
    })
}

#[derive(Clone, Copy, Debug)]
pub struct PageDemo {
    id: PageDemoId,
    page: PlayerPage,
    context: fn() -> Result<PageDemoContext, PageDemoError>,
    initial_route: fn() -> Result<PlayerRoute, PageDemoError>,
}

impl PageDemo {
    pub const fn new(
        id: PageDemoId,
        page: PlayerPage,
        context: fn() -> Result<PageDemoContext, PageDemoError>,
        initial_route: fn() -> Result<PlayerRoute, PageDemoError>,
    ) -> Self {
        Self {
            id,
            page,
            context,
            initial_route,
        }
    }

    pub const fn id(self) -> PageDemoId {
        self.id
    }

    pub fn initial_state(self) -> Result<PageState, PageDemoError> {
        (self.initial_route)().map(PageState::from_route)
    }

    pub const fn page(self) -> PlayerPage {
        self.page
    }

    /// 构建该 demo 固有的只读产品快照。
    pub fn context(self) -> Result<PageDemoContext, PageDemoError> {
        (self.context)()
    }

    /// 使用标准小镇 fixture 构建一个无需 renderer 的页面模型。
    pub fn model(self) -> Result<PageModel, PageDemoError> {
        let context = self.context()?;
        let route = self.initial_state()?.route().clone();
        project_demo_page(&context, &route).map_err(PageDemoError::Model)
    }
}

const WORLD_STARTING_TOWN: PageDemo = PageDemo::new(
    PageDemoId::new("world-starting-town"),
    PlayerPage::World,
    PageDemoContext::standard,
    world_route,
);
const WORLD_STARTING_DOWN: PageDemo = PageDemo::new(
    PageDemoId::new("world-starting-down"),
    PlayerPage::World,
    PageDemoContext::standard,
    world_route,
);
const WORLD_PAUSE_MENU: PageDemo = PageDemo::new(
    PageDemoId::new("world-pause-menu"),
    PlayerPage::Pause,
    PageDemoContext::standard,
    pause_menu_route,
);
const PARTY_SINGLE_MEMBER: PageDemo = PageDemo::new(
    PageDemoId::new("party-single-member"),
    PlayerPage::Pause,
    PageDemoContext::gift_received,
    party_route,
);
const BAG_POTION_LIST: PageDemo = PageDemo::new(
    PageDemoId::new("bag-potion-list"),
    PlayerPage::Pause,
    PageDemoContext::gift_received,
    bag_route,
);
const POKEDEX_SEEN_AND_UNSEEN: PageDemo = PageDemo::new(
    PageDemoId::new("pokedex-seen-and-unseen"),
    PlayerPage::Pause,
    PageDemoContext::pokedex_demo,
    pokedex_route,
);
const TRAINER_CARD_STARTING_TOWN: PageDemo = PageDemo::new(
    PageDemoId::new("trainer-card-starting-town"),
    PlayerPage::Pause,
    PageDemoContext::gift_received,
    trainer_card_route,
);
const SHOP_POTION_PREVIEW: PageDemo = PageDemo::new(
    PageDemoId::new("shop-potion-preview"),
    PlayerPage::Shop,
    PageDemoContext::standard,
    shop_potion_route,
);
const SAVE_CONFIRM_AVAILABLE: PageDemo = PageDemo::new(
    PageDemoId::new("save-confirm-available"),
    PlayerPage::SaveConfirm,
    PageDemoContext::standard,
    save_confirm_route,
);

fn world_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::World)
}

fn pause_menu_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::Pause {
        route: PauseRoute::Menu,
    })
}

fn party_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::Pause {
        route: PauseRoute::Party {
            selected: Some(CreatureId::new("starter-treecko-1").map_err(PageDemoError::Id)?),
        },
    })
}

fn bag_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::Pause {
        route: PauseRoute::Bag {
            category: BagFilter::All,
            selected: Some(ItemId::new("potion").map_err(PageDemoError::Id)?),
        },
    })
}

fn pokedex_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::Pause {
        route: PauseRoute::Pokedex {
            selected: NationalDexNumber::first(),
            stats_view: PokedexStatsView::Bars,
            selected_move: 0,
        },
    })
}

fn trainer_card_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::Pause {
        route: PauseRoute::TrainerCard,
    })
}

fn shop_potion_route() -> Result<PlayerRoute, PageDemoError> {
    let shop = ShopId::new("town-mart").map_err(PageDemoError::Id)?;
    let potion = ItemId::new("potion").map_err(PageDemoError::Id)?;
    Ok(PlayerRoute::Shop {
        shop,
        selected_item: Some(potion),
        quantity: 1,
    })
}

fn save_confirm_route() -> Result<PlayerRoute, PageDemoError> {
    Ok(PlayerRoute::SaveConfirm)
}

const PAGE_DEMOS: [PageDemo; 9] = [
    WORLD_STARTING_TOWN,
    WORLD_STARTING_DOWN,
    WORLD_PAUSE_MENU,
    PARTY_SINGLE_MEMBER,
    BAG_POTION_LIST,
    POKEDEX_SEEN_AND_UNSEEN,
    TRAINER_CARD_STARTING_TOWN,
    SHOP_POTION_PREVIEW,
    SAVE_CONFIRM_AVAILABLE,
];

pub fn page_demos() -> &'static [PageDemo] {
    &PAGE_DEMOS
}

pub fn demo_for(id: PageDemoId) -> Option<PageDemo> {
    page_demos().iter().copied().find(|demo| demo.id() == id)
}

/// 供二进制和开发工具将受限文本参数解析为已登记的 demo。
pub fn demo_named(value: &str) -> Option<PageDemo> {
    page_demos()
        .iter()
        .copied()
        .find(|demo| demo.id().as_str() == value)
}

/// 标准 demo 所需的静态内容和只读产品快照。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageDemoContext {
    content: ThinSliceContent,
    snapshot: ProductSnapshot,
    pokedex_display_mode: PokedexDisplayMode,
}

impl PageDemoContext {
    pub fn standard() -> Result<Self, PageDemoError> {
        let data = CurrentDataSet::embedded().map_err(PageDemoError::Data)?;
        let content = ThinSliceContent::standard().map_err(PageDemoError::Content)?;
        let session = ProductSession::new(data, content.clone()).map_err(PageDemoError::Product)?;
        Ok(Self {
            content,
            snapshot: session.snapshot(),
            pokedex_display_mode: PokedexDisplayMode::ProductFacts,
        })
    }

    /// 通过产品命令取得教授赠礼后的快照，供队伍和背包页面复用。
    pub fn gift_received() -> Result<Self, PageDemoError> {
        let data = CurrentDataSet::embedded().map_err(PageDemoError::Data)?;
        let content = ThinSliceContent::standard().map_err(PageDemoError::Content)?;
        let session = ProductSession::new(data, content.clone()).map_err(PageDemoError::Product)?;
        let (session, moved) = session.transition(ProductCommand::Move(Direction::Up));
        moved.map_err(PageDemoError::Product)?;
        let (session, gifted) = session.transition(ProductCommand::InteractFront);
        gifted.map_err(PageDemoError::Product)?;
        Ok(Self {
            content,
            snapshot: session.snapshot(),
            pokedex_display_mode: PokedexDisplayMode::ProductFacts,
        })
    }

    pub fn pokedex_demo() -> Result<Self, PageDemoError> {
        let data = CurrentDataSet::embedded().map_err(PageDemoError::Data)?;
        let content = ThinSliceContent::standard().map_err(PageDemoError::Content)?;
        let session = ProductSession::new(data, content.clone()).map_err(PageDemoError::Product)?;
        Ok(Self {
            content,
            snapshot: session.snapshot(),
            pokedex_display_mode: PokedexDisplayMode::DemoAllOpen,
        })
    }

    pub const fn content(&self) -> &ThinSliceContent {
        &self.content
    }

    pub const fn snapshot(&self) -> &ProductSnapshot {
        &self.snapshot
    }
}

#[derive(Debug)]
pub enum PageDemoError {
    Id(GameIdError),
    Dex(NationalDexError),
    Data(DataLoadError),
    Content(ContentError),
    Product(ProductError),
    Model(PageModelError),
}

impl fmt::Display for PageDemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(error) => write!(formatter, "standard demo id is invalid: {error:?}"),
            Self::Dex(error) => write!(formatter, "standard demo dex number is invalid: {error}"),
            Self::Data(error) => write!(formatter, "standard data unavailable: {error}"),
            Self::Content(error) => write!(formatter, "standard content unavailable: {error:?}"),
            Self::Product(error) => write!(formatter, "standard product unavailable: {error:?}"),
            Self::Model(error) => write!(formatter, "demo page model unavailable: {error}"),
        }
    }
}

impl Error for PageDemoError {}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
