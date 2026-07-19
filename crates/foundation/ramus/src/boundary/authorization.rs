use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::model::{Capability, MethodName, NodePath, PrincipalId};
use crate::policy::{CapabilityGrant, CapabilityView, allows};

struct AuthorityMarker;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityGeneration(u64);

impl CapabilityGeneration {
    pub const fn initial() -> Self {
        Self(1)
    }

    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

#[derive(Clone)]
pub struct Principal {
    id: PrincipalId,
    marker: Arc<AuthorityMarker>,
}

impl Principal {
    pub fn id(&self) -> &PrincipalId {
        &self.id
    }
}

pub struct EffectPermit {
    principal: PrincipalId,
    path: NodePath,
    method: MethodName,
    capability: Capability,
}

impl EffectPermit {
    pub fn principal(&self) -> &PrincipalId {
        &self.principal
    }

    pub fn path(&self) -> &NodePath {
        &self.path
    }

    pub fn method(&self) -> &MethodName {
        &self.method
    }

    pub const fn capability(&self) -> Capability {
        self.capability
    }
}

pub struct AuthorizationService {
    state: Arc<Mutex<AuthorizationState>>,
    marker: Arc<AuthorityMarker>,
}

#[derive(Clone)]
pub struct AuthorizationChecker {
    state: Arc<Mutex<AuthorizationState>>,
    marker: Arc<AuthorityMarker>,
}

#[derive(Clone)]
pub struct AuthorizationRevoker {
    state: Arc<Mutex<AuthorizationState>>,
    marker: Arc<AuthorityMarker>,
}

pub struct AuthorizationSession<'a> {
    state: MutexGuard<'a, AuthorizationState>,
    principal: &'a Principal,
    missing_authorization: PrincipalAuthorization,
}

struct AuthorizationState {
    principals: BTreeMap<PrincipalId, PrincipalAuthorization>,
}

#[derive(Clone, Default)]
struct PrincipalAuthorization {
    generation: CapabilityGeneration,
    grants: BTreeSet<CapabilityGrant>,
}

impl AuthorizationService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AuthorizationState {
                principals: BTreeMap::new(),
            })),
            marker: Arc::new(AuthorityMarker),
        }
    }

    pub fn create_principal(&self, id: impl Into<String>) -> Result<Principal, PrincipalError> {
        let id = PrincipalId::new(id).map_err(|_| PrincipalError::InvalidId)?;
        let mut state = lock_state(&self.state)?;
        if state.principals.contains_key(&id) {
            return Err(PrincipalError::DuplicateId);
        }
        state
            .principals
            .insert(id.clone(), PrincipalAuthorization::default());
        Ok(Principal {
            id,
            marker: Arc::clone(&self.marker),
        })
    }

    pub fn checker(&self) -> AuthorizationChecker {
        AuthorizationChecker {
            state: Arc::clone(&self.state),
            marker: Arc::clone(&self.marker),
        }
    }

    pub fn revoker(&self) -> AuthorizationRevoker {
        AuthorizationRevoker {
            state: Arc::clone(&self.state),
            marker: Arc::clone(&self.marker),
        }
    }

    pub fn generation(
        &self,
        principal: &Principal,
    ) -> Result<CapabilityGeneration, PrincipalError> {
        self.verify_principal(principal)?;
        let state = lock_state(&self.state)?;
        state
            .principals
            .get(&principal.id)
            .map(|authorization| authorization.generation)
            .ok_or(PrincipalError::UnknownPrincipal)
    }

    pub fn grant(
        &self,
        principal: &Principal,
        path: NodePath,
        method: Option<MethodName>,
        capability: Capability,
    ) -> Result<(), PrincipalError> {
        self.verify_principal(principal)?;
        let mut state = lock_state(&self.state)?;
        let grant = CapabilityGrant {
            path,
            method,
            capability,
        };
        let authorization = state
            .principals
            .get_mut(&principal.id)
            .ok_or(PrincipalError::UnknownPrincipal)?;
        if !authorization.grants.contains(&grant) {
            let generation = authorization
                .generation
                .next()
                .ok_or(PrincipalError::GenerationExhausted)?;
            authorization.grants.insert(grant);
            authorization.generation = generation;
        }
        Ok(())
    }

    pub fn revoke_all(&self, principal: &Principal) -> Result<(), PrincipalError> {
        self.verify_principal(principal)?;
        let mut state = lock_state(&self.state)?;
        let authorization = state
            .principals
            .get_mut(&principal.id)
            .ok_or(PrincipalError::UnknownPrincipal)?;
        if !authorization.grants.is_empty() {
            let generation = authorization
                .generation
                .next()
                .ok_or(PrincipalError::GenerationExhausted)?;
            authorization.grants.clear();
            authorization.generation = generation;
        }
        Ok(())
    }

    pub fn session<'a>(
        &'a self,
        principal: &'a Principal,
    ) -> Result<AuthorizationSession<'a>, PrincipalError> {
        self.verify_principal(principal)?;
        let state = lock_state(&self.state)?;
        if !state.principals.contains_key(&principal.id) {
            return Err(PrincipalError::UnknownPrincipal);
        }
        Ok(AuthorizationSession {
            state,
            principal,
            missing_authorization: PrincipalAuthorization::default(),
        })
    }

    fn verify_principal(&self, principal: &Principal) -> Result<(), PrincipalError> {
        Arc::ptr_eq(&self.marker, &principal.marker)
            .then_some(())
            .ok_or(PrincipalError::ForeignAuthority)
    }
}

impl AuthorizationSession<'_> {
    pub fn view(&self) -> CapabilityView<'_> {
        let authorization = self
            .state
            .principals
            .get(&self.principal.id)
            .unwrap_or(&self.missing_authorization);
        CapabilityView::new(
            self.principal,
            authorization.generation,
            &authorization.grants,
        )
    }
}

impl AuthorizationChecker {
    pub(crate) fn issue_permit(
        &self,
        principal: &Principal,
        path: &NodePath,
        method: &MethodName,
        capability: Capability,
        expected_generation: CapabilityGeneration,
    ) -> Option<EffectPermit> {
        self.issue_permit_after_lock(
            principal,
            path,
            method,
            capability,
            expected_generation,
            || {},
        )
    }

    fn issue_permit_after_lock(
        &self,
        principal: &Principal,
        path: &NodePath,
        method: &MethodName,
        capability: Capability,
        expected_generation: CapabilityGeneration,
        after_lock: impl FnOnce(),
    ) -> Option<EffectPermit> {
        self.verify_principal(principal).ok()?;
        let state = self.state.lock().ok()?;
        after_lock();
        let authorization = state.principals.get(&principal.id)?;
        let granted = allows(&authorization.grants, path, method, capability);
        (authorization.generation == expected_generation && granted).then(|| EffectPermit {
            principal: principal.id.clone(),
            path: path.clone(),
            method: method.clone(),
            capability,
        })
    }

    fn verify_principal(&self, principal: &Principal) -> Result<(), PrincipalError> {
        Arc::ptr_eq(&self.marker, &principal.marker)
            .then_some(())
            .ok_or(PrincipalError::ForeignAuthority)
    }
}

impl AuthorizationRevoker {
    pub fn revoke_all(&self, principal: &Principal) -> Result<(), PrincipalError> {
        self.revoke_all_after_lock(principal, || {})
    }

    fn revoke_all_after_lock(
        &self,
        principal: &Principal,
        after_lock: impl FnOnce(),
    ) -> Result<(), PrincipalError> {
        self.verify_principal(principal)?;
        let mut state = lock_state(&self.state)?;
        after_lock();
        let authorization = state
            .principals
            .get_mut(&principal.id)
            .ok_or(PrincipalError::UnknownPrincipal)?;
        if !authorization.grants.is_empty() {
            let generation = authorization
                .generation
                .next()
                .ok_or(PrincipalError::GenerationExhausted)?;
            authorization.grants.clear();
            authorization.generation = generation;
        }
        Ok(())
    }

    fn verify_principal(&self, principal: &Principal) -> Result<(), PrincipalError> {
        Arc::ptr_eq(&self.marker, &principal.marker)
            .then_some(())
            .ok_or(PrincipalError::ForeignAuthority)
    }
}

impl Default for AuthorizationService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrincipalError {
    InvalidId,
    DuplicateId,
    ForeignAuthority,
    UnknownPrincipal,
    StateUnavailable,
    GenerationExhausted,
}

fn lock_state(
    state: &Mutex<AuthorizationState>,
) -> Result<MutexGuard<'_, AuthorizationState>, PrincipalError> {
    state.lock().map_err(|_| PrincipalError::StateUnavailable)
}

impl Default for CapabilityGeneration {
    fn default() -> Self {
        Self::initial()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/boundary/authorization.rs"]
mod tests;
