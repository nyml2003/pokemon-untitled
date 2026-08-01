use crate::enums::{Ability, PokemonType};
use crate::error::ValidationError;
use crate::id::{FormId, NationalDexId};
use crate::stats::StatBlock;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Species {
    /// 物种显示名。
    pub name: String,
    /// 六维种族值：HP、攻击、防御、特攻、特防、速度。
    pub base_stats: StatBlock<u16>,
    /// 全国图鉴编号。
    pub national_dex_id: NationalDexId,
    /// 变体编号。
    pub form_id: FormId,
    /// 属性列表。
    pub types: Vec<PokemonType>,
    /// 默认特性列表。
    pub default_abilities: Vec<Ability>,
}

impl Species {
    pub fn new(
        name: impl Into<String>,
        base_stats: StatBlock<u16>,
        national_dex_id: NationalDexId,
        form_id: FormId,
        types: Vec<PokemonType>,
        default_abilities: Vec<Ability>,
    ) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonName);
        }
        if types.is_empty() {
            return Err(ValidationError::EmptySpeciesType);
        }
        Ok(Self {
            name,
            base_stats,
            national_dex_id,
            form_id,
            types,
            default_abilities,
        })
    }

    /// 物种显示名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 六维种族值。
    pub const fn base_stats(&self) -> StatBlock<u16> {
        self.base_stats
    }

    /// 全国图鉴编号。
    pub const fn national_dex_id(&self) -> NationalDexId {
        self.national_dex_id
    }

    /// 形态/变体编号。
    pub const fn form_id(&self) -> FormId {
        self.form_id
    }

    /// 属性列表。
    pub fn types(&self) -> &[PokemonType] {
        &self.types
    }

    /// 默认特性列表。
    pub fn default_abilities(&self) -> &[Ability] {
        &self.default_abilities
    }
}
