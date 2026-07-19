use std::collections::BTreeSet;

use crate::boundary::{CapabilityGeneration, Principal};
use crate::model::{Capability, MethodName, NodePath};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CapabilityGrant {
    pub path: NodePath,
    pub method: Option<MethodName>,
    pub capability: Capability,
}

pub struct CapabilityView<'a> {
    principal: &'a Principal,
    generation: CapabilityGeneration,
    grants: &'a BTreeSet<CapabilityGrant>,
}

impl<'a> CapabilityView<'a> {
    pub(crate) fn new(
        principal: &'a Principal,
        generation: CapabilityGeneration,
        grants: &'a BTreeSet<CapabilityGrant>,
    ) -> Self {
        Self {
            principal,
            generation,
            grants,
        }
    }

    pub fn principal(&self) -> &Principal {
        self.principal
    }

    pub const fn generation(&self) -> CapabilityGeneration {
        self.generation
    }

    pub(crate) fn allows(
        &self,
        path: &NodePath,
        method: &MethodName,
        capability: Capability,
    ) -> bool {
        allows(self.grants, path, method, capability)
    }
}

pub(crate) fn allows(
    grants: &BTreeSet<CapabilityGrant>,
    path: &NodePath,
    method: &MethodName,
    capability: Capability,
) -> bool {
    grants.iter().any(|grant| {
        if grant.capability != capability || !grant.path.is_prefix_of(path) {
            return false;
        }
        match &grant.method {
            Some(granted) => granted == method,
            None => true,
        }
    })
}
