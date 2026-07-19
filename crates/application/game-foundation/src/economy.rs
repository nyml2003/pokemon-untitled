use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ItemId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Money(u32);

impl Money {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn amount(self) -> u32 {
        self.0
    }

    pub fn credit(self, amount: Money) -> Result<Self, EconomyError> {
        self.0
            .checked_add(amount.0)
            .map(Self)
            .ok_or(EconomyError::MoneyOverflow)
    }

    pub fn spend(self, amount: Money) -> Result<Self, EconomyError> {
        self.0
            .checked_sub(amount.0)
            .map(Self)
            .ok_or(EconomyError::InsufficientMoney {
                required: amount,
                available: self,
            })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ItemCategory {
    Medicine,
    Key,
    General,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ItemDefinition {
    id: ItemId,
    category: ItemCategory,
    stack_limit: u16,
}

impl ItemDefinition {
    pub fn new(id: ItemId, category: ItemCategory, stack_limit: u16) -> Result<Self, EconomyError> {
        if stack_limit == 0 {
            return Err(EconomyError::ZeroStackLimit { item: id });
        }
        Ok(Self {
            id,
            category,
            stack_limit,
        })
    }

    pub fn id(&self) -> &ItemId {
        &self.id
    }

    pub const fn category(&self) -> ItemCategory {
        self.category
    }

    pub const fn stack_limit(&self) -> u16 {
        self.stack_limit
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShopListing {
    item: ItemId,
    unit_price: Money,
}

impl ShopListing {
    pub fn new(item: ItemId, unit_price: Money) -> Self {
        Self { item, unit_price }
    }

    pub fn item(&self) -> &ItemId {
        &self.item
    }

    pub const fn unit_price(&self) -> Money {
        self.unit_price
    }

    pub fn total_price(&self, quantity: u16) -> Result<Money, EconomyError> {
        let amount = self
            .unit_price
            .amount()
            .checked_mul(u32::from(quantity))
            .ok_or(EconomyError::MoneyOverflow)?;
        Ok(Money::new(amount))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Inventory {
    capacity: u16,
    entries: BTreeMap<ItemId, u16>,
}

impl Inventory {
    pub fn new(capacity: u16) -> Result<Self, EconomyError> {
        if capacity == 0 {
            return Err(EconomyError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            entries: BTreeMap::new(),
        })
    }

    pub const fn capacity(&self) -> u16 {
        self.capacity
    }

    pub fn entries(&self) -> &BTreeMap<ItemId, u16> {
        &self.entries
    }

    pub fn quantity(&self, item: &ItemId) -> u16 {
        self.entries.get(item).copied().unwrap_or(0)
    }

    pub fn add(&mut self, definition: &ItemDefinition, quantity: u16) -> Result<(), EconomyError> {
        if quantity == 0 {
            return Err(EconomyError::ZeroQuantity);
        }
        let existing = self.quantity(definition.id());
        if existing == 0 && self.entries.len() >= usize::from(self.capacity) {
            return Err(EconomyError::CapacityExceeded {
                capacity: self.capacity,
            });
        }
        let attempted =
            existing
                .checked_add(quantity)
                .ok_or_else(|| EconomyError::StackLimitExceeded {
                    item: definition.id().clone(),
                    limit: definition.stack_limit(),
                    attempted: u16::MAX,
                })?;
        if attempted > definition.stack_limit() {
            return Err(EconomyError::StackLimitExceeded {
                item: definition.id().clone(),
                limit: definition.stack_limit(),
                attempted,
            });
        }
        self.entries.insert(definition.id().clone(), attempted);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EconomyError {
    ZeroCapacity,
    ZeroQuantity,
    ZeroStackLimit {
        item: ItemId,
    },
    CapacityExceeded {
        capacity: u16,
    },
    StackLimitExceeded {
        item: ItemId,
        limit: u16,
        attempted: u16,
    },
    InsufficientMoney {
        required: Money,
        available: Money,
    },
    MoneyOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: &str, stack_limit: u16) -> Result<ItemDefinition, String> {
        let id = ItemId::new(value).map_err(|error| format!("item id: {error:?}"))?;
        ItemDefinition::new(id, ItemCategory::General, stack_limit)
            .map_err(|error| format!("item definition: {error:?}"))
    }

    #[test]
    fn inventory_enforces_capacity_and_stack_limits() -> Result<(), String> {
        let potion = item("potion", 2)?;
        let antidote = item("antidote", 5)?;
        let mut inventory = Inventory::new(1).map_err(|error| format!("inventory: {error:?}"))?;
        inventory
            .add(&potion, 2)
            .map_err(|error| format!("add potion: {error:?}"))?;
        assert!(matches!(
            inventory.add(&potion, 1),
            Err(EconomyError::StackLimitExceeded { .. })
        ));
        assert!(matches!(
            inventory.add(&antidote, 1),
            Err(EconomyError::CapacityExceeded { capacity: 1 })
        ));
        assert_eq!(inventory.quantity(potion.id()), 2);
        assert_eq!(inventory.quantity(antidote.id()), 0);
        Ok(())
    }

    #[test]
    fn money_rejects_an_insufficient_balance() {
        assert!(matches!(
            Money::new(29).spend(Money::new(30)),
            Err(EconomyError::InsufficientMoney {
                required,
                available,
            }) if required == Money::new(30) && available == Money::new(29)
        ));
    }
}
