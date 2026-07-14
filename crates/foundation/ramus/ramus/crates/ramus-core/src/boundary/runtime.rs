use std::collections::{BTreeMap, btree_map::Entry};
use std::sync::Arc;

use super::{AuthorizationChecker, EffectPermit};
use crate::catalog::Catalog;
use crate::execution::{
    ExecutionError, ExecutionFailure, ExecutionReport, ProviderError, ProviderRequest,
    preflight_plan,
};
use crate::model::ProviderId;
use crate::plan::TypedPlan;
use crate::schema::Value;

pub trait Provider: Send + Sync {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError>;
}

pub struct Runtime {
    catalog: Arc<Catalog>,
    authorization: AuthorizationChecker,
    providers: BTreeMap<ProviderId, Arc<dyn Provider>>,
}

impl Runtime {
    pub fn new(catalog: Arc<Catalog>, authorization: AuthorizationChecker) -> Self {
        Self {
            catalog,
            authorization,
            providers: BTreeMap::new(),
        }
    }

    pub fn bind_provider(
        &mut self,
        provider_id: ProviderId,
        provider: Arc<dyn Provider>,
    ) -> Result<(), RuntimeConfigurationError> {
        match self.providers.entry(provider_id) {
            Entry::Vacant(entry) => {
                entry.insert(provider);
                Ok(())
            }
            Entry::Occupied(_) => Err(RuntimeConfigurationError::DuplicateProvider),
        }
    }

    pub fn execute(&self, plan: TypedPlan) -> Result<ExecutionReport, ExecutionFailure> {
        let first_call = &plan.calls()[0];
        let Some(first_permit) = self.authorization.issue_permit(
            plan.principal_handle(),
            &first_call.path,
            &first_call.method,
            first_call.effect.into(),
            plan.capability_generation(),
        ) else {
            return Err(ExecutionFailure::before_any(
                ExecutionError::AuthorizationRevoked,
            ));
        };
        preflight_plan(&self.catalog, &plan)?;
        let mut outputs = Vec::with_capacity(plan.len());
        let mut next_permit = Some(first_permit);
        for (call_index, call) in plan.calls().iter().enumerate() {
            let permit = match next_permit.take() {
                Some(permit) => permit,
                None => {
                    let Some(permit) = self.authorization.issue_permit(
                        plan.principal_handle(),
                        &call.path,
                        &call.method,
                        call.effect.into(),
                        plan.capability_generation(),
                    ) else {
                        return Err(ExecutionFailure::new(
                            call_index,
                            outputs,
                            ExecutionError::AuthorizationRevoked,
                        ));
                    };
                    permit
                }
            };
            let Some(provider) = self.providers.get(&call.provider_id) else {
                return Err(ExecutionFailure::new(
                    call_index,
                    outputs,
                    ExecutionError::ProviderUnavailable,
                ));
            };
            let request = ProviderRequest::from(call);
            let output = provider.execute(permit, &request).map_err(|error| {
                ExecutionFailure::new(call_index, outputs.clone(), ExecutionError::Provider(error))
            })?;
            outputs.push(output);
        }
        Ok(ExecutionReport { outputs })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeConfigurationError {
    DuplicateProvider,
}
