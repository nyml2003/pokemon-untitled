use std::collections::BTreeSet;
use std::time::Duration;

use game_data::{AbilityId, TypeId};
use game_page_model::{
    PokedexEntryModel, PokedexFilterIntent, PokedexMoveCategory, PokedexMoveModel,
};
use punctum_ui::{FormItem, FormItemKind, KeyboardForm, KeyboardFormError};

const RANGE_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PokedexFilterItem {
    Types,
    TypeMatch,
    Generations,
    HeightMinimum,
    HeightMaximum,
    WeightMinimum,
    WeightMaximum,
    Ability,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveFilterItem {
    Name,
    Types,
    Category,
    PowerMinimum,
    PowerMaximum,
    Accuracy,
    Priority,
    Reset,
}

pub fn pokedex_filter_form()
-> Result<KeyboardForm<PokedexFilterItem>, KeyboardFormError<PokedexFilterItem>> {
    KeyboardForm::try_new([
        FormItem::new(PokedexFilterItem::Types, FormItemKind::Group),
        FormItem::new(PokedexFilterItem::TypeMatch, FormItemKind::Group),
        FormItem::new(PokedexFilterItem::Generations, FormItemKind::Group),
        FormItem::new(PokedexFilterItem::HeightMinimum, FormItemKind::Field),
        FormItem::new(PokedexFilterItem::HeightMaximum, FormItemKind::Field),
        FormItem::new(PokedexFilterItem::WeightMinimum, FormItemKind::Field),
        FormItem::new(PokedexFilterItem::WeightMaximum, FormItemKind::Field),
        FormItem::new(PokedexFilterItem::Ability, FormItemKind::Select),
        FormItem::new(PokedexFilterItem::Reset, FormItemKind::Command),
    ])
}

pub fn move_filter_form() -> Result<KeyboardForm<MoveFilterItem>, KeyboardFormError<MoveFilterItem>>
{
    KeyboardForm::try_new([
        FormItem::new(MoveFilterItem::Name, FormItemKind::Field),
        FormItem::new(MoveFilterItem::Types, FormItemKind::Group),
        FormItem::new(MoveFilterItem::Category, FormItemKind::Group),
        FormItem::new(MoveFilterItem::PowerMinimum, FormItemKind::Field),
        FormItem::new(MoveFilterItem::PowerMaximum, FormItemKind::Field),
        FormItem::new(MoveFilterItem::Accuracy, FormItemKind::Select),
        FormItem::new(MoveFilterItem::Priority, FormItemKind::Field),
        FormItem::new(MoveFilterItem::Reset, FormItemKind::Command),
    ])
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeMatch {
    #[default]
    Any,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PokedexFilterModel {
    pub type_ids: BTreeSet<TypeId>,
    pub type_match: TypeMatch,
    pub generations: BTreeSet<u8>,
    pub height_decimeters: (Option<u16>, Option<u16>),
    pub weight_hectograms: (Option<u16>, Option<u16>),
    pub ability: Option<AbilityId>,
    ability_query: String,
    height_draft: NumericRangeDraft,
    weight_draft: NumericRangeDraft,
}

impl PokedexFilterModel {
    pub fn matches(&self, entry: &PokedexEntryModel) -> bool {
        if !generation_matches(entry.number.value(), &self.generations) {
            return false;
        }
        if !entry.known {
            return self.type_ids.is_empty()
                && self.height_decimeters == (None, None)
                && self.weight_hectograms == (None, None)
                && self.ability.is_none();
        }
        let types_match = self.type_ids.is_empty()
            || match self.type_match {
                TypeMatch::Any => entry.type_ids.iter().any(|id| self.type_ids.contains(id)),
                TypeMatch::All => self.type_ids.iter().all(|id| entry.type_ids.contains(id)),
            };
        types_match
            && range_matches(entry.height_decimeters, self.height_decimeters)
            && range_matches(entry.weight_hectograms, self.weight_hectograms)
            && self
                .ability
                .is_none_or(|id| entry.abilities.iter().any(|ability| ability.id == id))
    }

    pub fn height_draft(&self) -> (&str, &str) {
        self.height_draft.values()
    }

    pub fn weight_draft(&self) -> (&str, &str) {
        self.weight_draft.values()
    }

    pub fn ability_query(&self) -> &str {
        &self.ability_query
    }

    pub fn apply(&mut self, intent: &PokedexFilterIntent) -> bool {
        match intent {
            PokedexFilterIntent::ToggleType(id) => toggle_set(&mut self.type_ids, *id),
            PokedexFilterIntent::SetTypeMatchAll(all) => {
                let next = if *all { TypeMatch::All } else { TypeMatch::Any };
                let changed = self.type_match != next;
                self.type_match = next;
                changed
            }
            PokedexFilterIntent::ToggleGeneration(generation) => {
                toggle_set(&mut self.generations, *generation)
            }
            PokedexFilterIntent::SetHeightMinimum(value) => self.height_draft.set_minimum(value),
            PokedexFilterIntent::SetHeightMaximum(value) => self.height_draft.set_maximum(value),
            PokedexFilterIntent::SetWeightMinimum(value) => self.weight_draft.set_minimum(value),
            PokedexFilterIntent::SetWeightMaximum(value) => self.weight_draft.set_maximum(value),
            PokedexFilterIntent::SetAbilityQuery(query) => {
                let changed = self.ability_query != *query;
                self.ability_query.clone_from(query);
                changed
            }
            PokedexFilterIntent::SelectAbility(ability) => {
                let changed = self.ability != *ability;
                self.ability = *ability;
                changed
            }
            PokedexFilterIntent::ResetPokedex => self.reset(),
            PokedexFilterIntent::ResetMove
            | PokedexFilterIntent::SetMoveName(_)
            | PokedexFilterIntent::ToggleMoveType(_)
            | PokedexFilterIntent::SelectMoveCategory(_)
            | PokedexFilterIntent::SetMovePowerMinimum(_)
            | PokedexFilterIntent::SetMovePowerMaximum(_)
            | PokedexFilterIntent::SelectMoveAccuracy(_)
            | PokedexFilterIntent::ToggleMovePriority => false,
        }
    }

    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let height = self.height_draft.advance(elapsed, 10).map(|range| {
            let changed = self.height_decimeters != range;
            self.height_decimeters = range;
            changed
        });
        let weight = self.weight_draft.advance(elapsed, 10).map(|range| {
            let changed = self.weight_hectograms != range;
            self.weight_hectograms = range;
            changed
        });
        height.is_some_and(|changed| changed) || weight.is_some_and(|changed| changed)
    }

    pub fn next_delay(&self) -> Option<Duration> {
        self.height_draft
            .next_delay()
            .into_iter()
            .chain(self.weight_draft.next_delay())
            .min()
    }

    fn reset(&mut self) -> bool {
        let next = Self::default();
        let changed = *self != next;
        *self = next;
        changed
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MoveFilterModel {
    pub name_query: String,
    pub type_ids: BTreeSet<TypeId>,
    pub category: Option<PokedexMoveCategory>,
    pub power: (Option<u16>, Option<u16>),
    pub accuracy: Option<Option<u8>>,
    pub priority_only: bool,
    power_draft: NumericRangeDraft,
}

impl MoveFilterModel {
    pub fn matches(&self, item: &PokedexMoveModel) -> bool {
        let query = self.name_query.trim();
        (query.is_empty() || item.name.to_lowercase().contains(&query.to_lowercase()))
            && (self.type_ids.is_empty() || self.type_ids.contains(&item.type_id))
            && self
                .category
                .is_none_or(|category| item.category == category)
            && range_matches(item.power, self.power)
            && self
                .accuracy
                .is_none_or(|accuracy| item.accuracy == accuracy)
            && (!self.priority_only || item.priority != 0)
    }

    pub fn power_draft(&self) -> (&str, &str) {
        self.power_draft.values()
    }

    pub fn apply(&mut self, intent: &PokedexFilterIntent) -> bool {
        match intent {
            PokedexFilterIntent::SetMoveName(name) => {
                let changed = self.name_query != *name;
                self.name_query.clone_from(name);
                changed
            }
            PokedexFilterIntent::ToggleMoveType(id) => toggle_set(&mut self.type_ids, *id),
            PokedexFilterIntent::SelectMoveCategory(category) => {
                let changed = self.category != *category;
                self.category = *category;
                changed
            }
            PokedexFilterIntent::SetMovePowerMinimum(value) => self.power_draft.set_minimum(value),
            PokedexFilterIntent::SetMovePowerMaximum(value) => self.power_draft.set_maximum(value),
            PokedexFilterIntent::SelectMoveAccuracy(accuracy) => {
                let changed = self.accuracy != *accuracy;
                self.accuracy = *accuracy;
                changed
            }
            PokedexFilterIntent::ToggleMovePriority => {
                self.priority_only = !self.priority_only;
                true
            }
            PokedexFilterIntent::ResetMove => self.reset(),
            PokedexFilterIntent::ToggleType(_)
            | PokedexFilterIntent::SetTypeMatchAll(_)
            | PokedexFilterIntent::ToggleGeneration(_)
            | PokedexFilterIntent::SetHeightMinimum(_)
            | PokedexFilterIntent::SetHeightMaximum(_)
            | PokedexFilterIntent::SetWeightMinimum(_)
            | PokedexFilterIntent::SetWeightMaximum(_)
            | PokedexFilterIntent::SetAbilityQuery(_)
            | PokedexFilterIntent::SelectAbility(_)
            | PokedexFilterIntent::ResetPokedex => false,
        }
    }

    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let power = self.power_draft.advance(elapsed, 1).map(|range| {
            let changed = self.power != range;
            self.power = range;
            changed
        });
        power.is_some_and(|changed| changed)
    }

    pub fn next_delay(&self) -> Option<Duration> {
        self.power_draft.next_delay()
    }

    fn reset(&mut self) -> bool {
        let next = Self::default();
        let changed = *self != next;
        *self = next;
        changed
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NumericRangeDraft {
    minimum: String,
    maximum: String,
    remaining: Option<Duration>,
}

impl NumericRangeDraft {
    fn values(&self) -> (&str, &str) {
        (&self.minimum, &self.maximum)
    }

    fn next_delay(&self) -> Option<Duration> {
        self.remaining
    }

    fn set_minimum(&mut self, value: &str) -> bool {
        if self.minimum == value {
            return false;
        }
        self.minimum = String::from(value);
        self.remaining = Some(RANGE_DEBOUNCE);
        true
    }

    fn set_maximum(&mut self, value: &str) -> bool {
        if self.maximum == value {
            return false;
        }
        self.maximum = String::from(value);
        self.remaining = Some(RANGE_DEBOUNCE);
        true
    }

    fn advance(&mut self, elapsed: Duration, scale: u16) -> Option<(Option<u16>, Option<u16>)> {
        let remaining = self.remaining?;
        if elapsed < remaining {
            let remaining = remaining.saturating_sub(elapsed);
            self.remaining = Some(remaining);
            return None;
        }
        self.remaining = None;
        let minimum = parse_scaled(&self.minimum, scale)?;
        let maximum = parse_scaled(&self.maximum, scale)?;
        (minimum
            .zip(maximum)
            .is_none_or(|(minimum, maximum)| minimum <= maximum))
        .then_some((minimum, maximum))
    }
}

fn toggle_set<T: Ord + Copy>(values: &mut BTreeSet<T>, value: T) -> bool {
    if values.remove(&value) {
        true
    } else {
        values.insert(value)
    }
}

fn parse_scaled(value: &str, scale: u16) -> Option<Option<u16>> {
    let value = value.trim();
    if value.is_empty() {
        return Some(None);
    }
    let (integer, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    if fraction.chars().count() > 1
        || (scale < 10 && !fraction.is_empty())
        || !integer.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
    {
        return None;
    }
    let integer = integer.parse::<u16>().ok()?;
    let fraction = fraction.chars().next().map_or(0, |digit| {
        digit.to_digit(10).map_or(0, |digit| digit as u16)
    });
    integer
        .checked_mul(scale)
        .and_then(|integer| integer.checked_add(fraction.saturating_mul(scale / 10)))
        .map(Some)
}

fn generation_matches(number: u16, generations: &BTreeSet<u8>) -> bool {
    generations.is_empty()
        || generations.iter().any(|generation| match generation {
            1 => (1..=151).contains(&number),
            2 => (152..=251).contains(&number),
            3 => (252..=386).contains(&number),
            _ => false,
        })
}

fn range_matches(value: Option<u16>, range: (Option<u16>, Option<u16>)) -> bool {
    let Some(value) = value else {
        return range == (None, None);
    };
    range.0.is_none_or(|minimum| value >= minimum) && range.1.is_none_or(|maximum| value <= maximum)
}
