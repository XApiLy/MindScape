use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::contracts::{
    CapabilityRequirement, ModelRunEventEnvelope, ModelRunRequest, ProviderError,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub text_input: bool,
    pub image_input: bool,
    pub tool_calling: bool,
    pub usage_reporting: bool,
    pub streaming: bool,
    pub context_window_tokens: Option<u64>,
}

impl ModelCapabilities {
    pub fn supports(&self, requirement: CapabilityRequirement) -> bool {
        match requirement {
            CapabilityRequirement::TextInput => self.text_input,
            CapabilityRequirement::ImageInput => self.image_input,
            CapabilityRequirement::ToolCalling => self.tool_calling,
            CapabilityRequirement::UsageReporting => self.usage_reporting,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub default_base_url: Option<String>,
    pub custom_base_url_allowed: bool,
    pub credential_required: bool,
    pub models: HashMap<String, ModelCapabilities>,
}

#[derive(Debug, Clone, Default)]
pub struct RunCancellation(Arc<std::sync::atomic::AtomicBool>);

impl RunCancellation {
    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
}

pub trait ProviderAdapter: Send + Sync {
    fn descriptor(&self) -> &ProviderDescriptor;

    fn run(
        &self,
        request: &ModelRunRequest,
        cancellation: &RunCancellation,
        emit: &mut dyn FnMut(ModelRunEventEnvelope),
    ) -> Result<(), ProviderError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProviderRuntimeError {
    #[error("provider is not registered: {0}")]
    ProviderNotRegistered(String),
    #[error("model is not registered for provider {provider_id}: {model_id}")]
    ModelNotRegistered {
        provider_id: String,
        model_id: String,
    },
    #[error("model {model_id} does not support capability {capability:?}")]
    CapabilityUnsupported {
        model_id: String,
        capability: CapabilityRequirement,
    },
    #[error("provider execution failed: {0:?}")]
    Provider(ProviderError),
}

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    adapters: HashMap<String, Arc<dyn ProviderAdapter>>,
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRegistry")
            .field("provider_ids", &self.adapters.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ProviderRegistry {
    pub fn register(&mut self, adapter: impl ProviderAdapter + 'static) -> Result<(), String> {
        let id = adapter.descriptor().id.trim();
        if id.is_empty() {
            return Err("provider id cannot be empty".into());
        }
        if self.adapters.contains_key(id) {
            return Err(format!("provider is already registered: {id}"));
        }
        self.adapters.insert(id.to_string(), Arc::new(adapter));
        Ok(())
    }

    pub fn descriptors(&self) -> Vec<ProviderDescriptor> {
        let mut descriptors = self
            .adapters
            .values()
            .map(|adapter| adapter.descriptor().clone())
            .collect::<Vec<_>>();
        descriptors.sort_by(|left, right| left.id.cmp(&right.id));
        descriptors
    }

    fn adapter(&self, provider_id: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_id).cloned()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProviderRuntime {
    registry: ProviderRegistry,
}

impl ProviderRuntime {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self { registry }
    }

    pub fn descriptors(&self) -> Vec<ProviderDescriptor> {
        self.registry.descriptors()
    }

    pub fn run(
        &self,
        request: &ModelRunRequest,
        cancellation: &RunCancellation,
        emit: &mut dyn FnMut(ModelRunEventEnvelope),
    ) -> Result<(), ProviderRuntimeError> {
        let adapter = self.registry.adapter(&request.provider_id).ok_or_else(|| {
            ProviderRuntimeError::ProviderNotRegistered(request.provider_id.clone())
        })?;
        let capabilities = adapter
            .descriptor()
            .models
            .get(&request.model_id)
            .ok_or_else(|| ProviderRuntimeError::ModelNotRegistered {
                provider_id: request.provider_id.clone(),
                model_id: request.model_id.clone(),
            })?;

        for capability in &request.capabilities {
            if !capabilities.supports(*capability) {
                return Err(ProviderRuntimeError::CapabilityUnsupported {
                    model_id: request.model_id.clone(),
                    capability: *capability,
                });
            }
        }

        adapter
            .run(request, cancellation, emit)
            .map_err(ProviderRuntimeError::Provider)
    }
}
