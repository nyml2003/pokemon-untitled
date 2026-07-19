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

    fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("capability generation exhausted"),
        )
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
        let mut state = self.state.lock().expect("authorization mutex poisoned");
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
        Ok(self
            .state
            .lock()
            .expect("authorization mutex poisoned")
            .principals
            .get(&principal.id)
            .map_or(CapabilityGeneration::initial(), |authorization| {
                authorization.generation
            }))
    }

    pub fn grant(
        &self,
        principal: &Principal,
        path: NodePath,
        method: Option<MethodName>,
        capability: Capability,
    ) -> Result<(), PrincipalError> {
        self.verify_principal(principal)?;
        let mut state = self.state.lock().expect("authorization mutex poisoned");
        let grant = CapabilityGrant {
            path,
            method,
            capability,
        };
        let authorization = state.principals.entry(principal.id.clone()).or_default();
        if authorization.grants.insert(grant) {
            authorization.generation = authorization.generation.next();
        }
        Ok(())
    }

    pub fn revoke_all(&self, principal: &Principal) -> Result<(), PrincipalError> {
        self.verify_principal(principal)?;
        let mut state = self.state.lock().expect("authorization mutex poisoned");
        let authorization = state.principals.entry(principal.id.clone()).or_default();
        if !authorization.grants.is_empty() {
            authorization.grants.clear();
            authorization.generation = authorization.generation.next();
        }
        Ok(())
    }

    pub fn session<'a>(
        &'a self,
        principal: &'a Principal,
    ) -> Result<AuthorizationSession<'a>, PrincipalError> {
        self.verify_principal(principal)?;
        Ok(AuthorizationSession {
            state: self.state.lock().expect("authorization mutex poisoned"),
            principal,
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
            .expect("issued principals remain registered");
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
        let state = self.state.lock().expect("authorization mutex poisoned");
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
        let mut state = self.state.lock().expect("authorization mutex poisoned");
        after_lock();
        let authorization = state
            .principals
            .get_mut(&principal.id)
            .expect("issued principals remain registered");
        if !authorization.grants.is_empty() {
            authorization.grants.clear();
            authorization.generation = authorization.generation.next();
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
}

impl Default for CapabilityGeneration {
    fn default() -> Self {
        Self::initial()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::{AuthorizationService, CapabilityGeneration, Principal};
    use crate::model::{Capability, MethodName, NodePath};

    #[test]
    fn permit_first_order_issues_exactly_one_pre_revoke_permit() {
        let (authorization, principal, path, method, generation) = fixture("permit-first");
        let permit_locked = Arc::new(Barrier::new(2));
        let release_permit = Arc::new(Barrier::new(2));
        let permit_thread = {
            let checker = authorization.checker();
            let principal = principal.clone();
            let path = path.clone();
            let method = method.clone();
            let permit_locked = Arc::clone(&permit_locked);
            let release_permit = Arc::clone(&release_permit);
            thread::spawn(move || {
                checker.issue_permit_after_lock(
                    &principal,
                    &path,
                    &method,
                    Capability::Invoke,
                    generation,
                    || {
                        permit_locked.wait();
                        release_permit.wait();
                    },
                )
            })
        };
        permit_locked.wait();

        let revoke_started = Arc::new(Barrier::new(2));
        let revoke_thread = {
            let revoker = authorization.revoker();
            let principal = principal.clone();
            let revoke_started = Arc::clone(&revoke_started);
            thread::spawn(move || {
                revoke_started.wait();
                revoker.revoke_all(&principal).unwrap();
            })
        };
        revoke_started.wait();
        release_permit.wait();

        assert!(permit_thread.join().unwrap().is_some());
        revoke_thread.join().unwrap();
        assert!(
            authorization
                .checker()
                .issue_permit(&principal, &path, &method, Capability::Invoke, generation,)
                .is_none()
        );
    }

    #[test]
    fn revoke_first_order_never_issues_a_post_revoke_permit() {
        let (authorization, principal, path, method, generation) = fixture("revoke-first");
        let revoke_locked = Arc::new(Barrier::new(2));
        let release_revoke = Arc::new(Barrier::new(2));
        let revoke_thread = {
            let revoker = authorization.revoker();
            let principal = principal.clone();
            let revoke_locked = Arc::clone(&revoke_locked);
            let release_revoke = Arc::clone(&release_revoke);
            thread::spawn(move || {
                revoker
                    .revoke_all_after_lock(&principal, || {
                        revoke_locked.wait();
                        release_revoke.wait();
                    })
                    .unwrap();
            })
        };
        revoke_locked.wait();

        let permit_started = Arc::new(Barrier::new(2));
        let permit_thread = {
            let checker = authorization.checker();
            let principal = principal.clone();
            let path = path.clone();
            let method = method.clone();
            let permit_started = Arc::clone(&permit_started);
            thread::spawn(move || {
                permit_started.wait();
                checker.issue_permit(&principal, &path, &method, Capability::Invoke, generation)
            })
        };
        permit_started.wait();
        release_revoke.wait();

        revoke_thread.join().unwrap();
        assert!(permit_thread.join().unwrap().is_none());
    }

    fn fixture(
        id: &str,
    ) -> (
        AuthorizationService,
        Principal,
        NodePath,
        MethodName,
        CapabilityGeneration,
    ) {
        let authorization = AuthorizationService::new();
        let principal = authorization.create_principal(id).unwrap();
        let path = NodePath::parse("/battle/turn").unwrap();
        let method = MethodName::new("submit").unwrap();
        authorization
            .grant(
                &principal,
                path.clone(),
                Some(method.clone()),
                Capability::Invoke,
            )
            .unwrap();
        let generation = authorization.generation(&principal).unwrap();
        (authorization, principal, path, method, generation)
    }
}
