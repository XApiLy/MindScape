use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::{State, ipc::Channel};

use crate::{
    adapters::{
        CredentialService,
        provider::{
            MockProvider, ProviderDescriptor, ProviderRegistry, ProviderRuntime,
            ProviderRuntimeError, RunCancellation,
        },
    },
    application::KernelService,
    domain::{
        AppendTurnInput, CompleteTurnInput, ContextSnapshot, Conversation, ConversationGraph,
        ConversationNode, CreateConversationInput, CredentialError, CredentialRef, KernelBootstrap,
        KernelError, SetCredentialInput, UpdateNodePositionInput,
        contracts::{ModelRunEventEnvelope, ModelRunRequest},
    },
};

#[derive(Debug, Clone)]
pub struct KernelState {
    service: KernelService,
    credentials: CredentialService,
    provider_runtime: ProviderRuntime,
    active_runs: Arc<Mutex<HashMap<String, RunCancellation>>>,
}

impl KernelState {
    pub fn new(service: KernelService, credentials: CredentialService) -> Self {
        let mut registry = ProviderRegistry::default();
        registry
            .register(MockProvider::standard())
            .expect("the built-in mock provider must register once");
        Self {
            service,
            credentials,
            provider_runtime: ProviderRuntime::new(registry),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub safe_message: String,
    pub retryable: bool,
}

impl From<KernelError> for CommandError {
    fn from(error: KernelError) -> Self {
        match error {
            KernelError::NotFound { entity, id } => Self {
                code: "notFound",
                safe_message: format!("{entity} not found: {id}"),
                retryable: false,
            },
            KernelError::Validation(message) => Self {
                code: "validation",
                safe_message: message,
                retryable: false,
            },
            KernelError::Integrity(message) => Self {
                code: "integrity",
                safe_message: message,
                retryable: false,
            },
            KernelError::Database(_) | KernelError::Serialization(_) | KernelError::Io(_) => Self {
                code: "storageUnavailable",
                safe_message: "Local data operation failed. Retry or restart MindScape.".into(),
                retryable: true,
            },
        }
    }
}

impl From<CredentialError> for CommandError {
    fn from(error: CredentialError) -> Self {
        match error {
            CredentialError::InvalidReference(message) => Self {
                code: "validation",
                safe_message: message,
                retryable: false,
            },
            CredentialError::NotFound => Self {
                code: "credentialNotFound",
                safe_message: "No credential is configured for this provider account.".into(),
                retryable: false,
            },
            CredentialError::Unavailable => Self {
                code: "credentialStoreUnavailable",
                safe_message: "The operating system credential store is unavailable.".into(),
                retryable: true,
            },
        }
    }
}

impl From<ProviderRuntimeError> for CommandError {
    fn from(error: ProviderRuntimeError) -> Self {
        Self {
            code: "providerRuntime",
            safe_message: error.to_string(),
            retryable: false,
        }
    }
}

#[tauri::command]
pub fn bootstrap_kernel(state: State<'_, KernelState>) -> CommandResult<KernelBootstrap> {
    state.service.bootstrap().map_err(Into::into)
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, KernelState>,
    input: CreateConversationInput,
) -> CommandResult<Conversation> {
    state.service.create_conversation(input).map_err(Into::into)
}

#[tauri::command]
pub fn load_conversation_graph(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<ConversationGraph> {
    state
        .service
        .load_conversation_graph(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn append_turn(
    state: State<'_, KernelState>,
    input: AppendTurnInput,
) -> CommandResult<ConversationNode> {
    state.service.append_turn(input).map_err(Into::into)
}

#[tauri::command]
pub fn complete_turn(
    state: State<'_, KernelState>,
    input: CompleteTurnInput,
) -> CommandResult<ConversationNode> {
    state.service.complete_turn(input).map_err(Into::into)
}

#[tauri::command]
pub fn get_context_snapshot(
    state: State<'_, KernelState>,
    snapshot_id: String,
) -> CommandResult<ContextSnapshot> {
    state
        .service
        .get_context_snapshot(&snapshot_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn update_node_position(
    state: State<'_, KernelState>,
    input: UpdateNodePositionInput,
) -> CommandResult<()> {
    state
        .service
        .update_node_position(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_provider_credential(
    state: State<'_, KernelState>,
    input: SetCredentialInput,
) -> CommandResult<()> {
    state
        .credentials
        .set(&input.reference, &input.secret)
        .map_err(Into::into)
}

#[tauri::command]
pub fn has_provider_credential(
    state: State<'_, KernelState>,
    reference: CredentialRef,
) -> CommandResult<bool> {
    state.credentials.exists(&reference).map_err(Into::into)
}

#[tauri::command]
pub fn delete_provider_credential(
    state: State<'_, KernelState>,
    reference: CredentialRef,
) -> CommandResult<()> {
    state.credentials.delete(&reference).map_err(Into::into)
}

#[tauri::command]
pub fn list_providers(state: State<'_, KernelState>) -> Vec<ProviderDescriptor> {
    state.provider_runtime.descriptors()
}

#[tauri::command]
pub async fn run_model(
    state: State<'_, KernelState>,
    request: ModelRunRequest,
    on_event: Channel<ModelRunEventEnvelope>,
) -> CommandResult<()> {
    state.service.create_model_run(&request)?;
    let cancellation = RunCancellation::default();
    {
        let mut active_runs = state.active_runs.lock().map_err(|_| CommandError {
            code: "runtimeUnavailable",
            safe_message: "The model runtime is unavailable.".into(),
            retryable: true,
        })?;
        if active_runs
            .insert(request.run_id.clone(), cancellation.clone())
            .is_some()
        {
            return Err(CommandError {
                code: "runAlreadyActive",
                safe_message: "This model run is already active.".into(),
                retryable: false,
            });
        }
    }

    let runtime = state.provider_runtime.clone();
    let service = state.service.clone();
    let run_id = request.run_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut callback_error = None;
        let runtime_result = runtime.run(&request, &cancellation, &mut |event| {
            if callback_error.is_some() {
                return;
            }
            if let Err(error) = service.record_model_run_event(&event) {
                callback_error = Some(CommandError::from(error));
                return;
            }
            if on_event.send(event).is_err() {
                cancellation.cancel();
                callback_error = Some(CommandError {
                    code: "eventChannelClosed",
                    safe_message: "The model event channel was closed.".into(),
                    retryable: true,
                });
            }
        });
        callback_error.map_or_else(|| runtime_result.map_err(Into::into), Err)
    })
    .await
    .map_err(|_| CommandError {
        code: "runtimeUnavailable",
        safe_message: "The model runtime stopped unexpectedly.".into(),
        retryable: true,
    })?;

    if let Ok(mut active_runs) = state.active_runs.lock() {
        active_runs.remove(&run_id);
    }
    result
}

#[tauri::command]
pub fn cancel_model_run(state: State<'_, KernelState>, run_id: String) -> CommandResult<bool> {
    let active_runs = state.active_runs.lock().map_err(|_| CommandError {
        code: "runtimeUnavailable",
        safe_message: "The model runtime is unavailable.".into(),
        retryable: true,
    })?;
    if let Some(cancellation) = active_runs.get(&run_id) {
        cancellation.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_storage_errors_are_not_exposed_to_the_frontend() {
        let error = CommandError::from(KernelError::Io(std::io::Error::other(
            "C:/private/user/path/mindscape.sqlite3",
        )));
        let json = serde_json::to_string(&error).expect("serialize command error");

        assert_eq!(error.code, "storageUnavailable");
        assert!(!json.contains("private/user/path"));
    }
}
