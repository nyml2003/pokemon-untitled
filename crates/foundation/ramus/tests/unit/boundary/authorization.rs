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
