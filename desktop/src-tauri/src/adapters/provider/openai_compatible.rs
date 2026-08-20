use std::{collections::HashMap, time::Duration};

use reqwest::{Client, StatusCode, redirect::Policy};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::CredentialService,
    domain::contracts::{
        FinishReason, ModelRunEvent, ModelRunEventEnvelope, ModelRunRequest, ModelUsage,
        ProviderError, ProviderErrorCategory, RUNTIME_CONTRACT_VERSION, RunCancelReason,
    },
    domain::{CredentialError, CredentialRef, blocks_plain_text, new_id, now_timestamp},
};

use super::{
    ModelCapabilities, ProviderAdapter, ProviderConnectionTestResult, ProviderDescriptor,
    RunCancellation, SseDecoder,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const FIRST_TOKEN_TIMEOUT: Duration = Duration::from_secs(20);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECTION_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MODELS_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: String,
    pub models_url: String,
    pub credential_account_id: String,
    pub models: Vec<String>,
    pub custom_base_url_allowed: bool,
    pub disable_thinking: bool,
}

impl OpenAiCompatibleConfig {
    pub fn deepseek() -> Self {
        Self {
            provider_id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/chat/completions".into(),
            models_url: "https://api.deepseek.com/models".into(),
            credential_account_id: "default".into(),
            models: vec!["deepseek-v4-flash".into()],
            custom_base_url_allowed: false,
            // DeepSeek V4 enables thinking by default. MindScape V1 only persists the
            // visible answer, so spending the entire output budget on reasoning would
            // otherwise produce a successful run with no displayable content.
            disable_thinking: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    descriptor: ProviderDescriptor,
    config: OpenAiCompatibleConfig,
    credentials: CredentialService,
    client: Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        config: OpenAiCompatibleConfig,
        credentials: CredentialService,
    ) -> Result<Self, ProviderError> {
        if !is_https_url(&config.base_url) || !is_https_url(&config.models_url) {
            return Err(provider_error(
                ProviderErrorCategory::InvalidRequest,
                "insecure_base_url",
                "Provider Base URL must use HTTPS.",
                false,
                None,
            ));
        }
        let models = config
            .models
            .iter()
            .map(|model| {
                (
                    model.clone(),
                    ModelCapabilities {
                        text_input: true,
                        image_input: false,
                        tool_calling: false,
                        usage_reporting: true,
                        streaming: true,
                        context_window_tokens: Some(1_000_000),
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(Policy::none())
            .build()
            .map_err(|_| network_error("http_client_build", false))?;
        let descriptor = ProviderDescriptor {
            id: config.provider_id.clone(),
            display_name: config.display_name.clone(),
            default_base_url: Some(config.base_url.clone()),
            custom_base_url_allowed: config.custom_base_url_allowed,
            credential_required: true,
            models,
        };
        Ok(Self {
            descriptor,
            config,
            credentials,
            client,
        })
    }

    fn credential(&self) -> Result<zeroize::Zeroizing<String>, ProviderError> {
        self.credentials
            .resolve(&CredentialRef {
                provider_id: self.config.provider_id.clone(),
                account_id: self.config.credential_account_id.clone(),
            })
            .map_err(map_credential_error)
    }
}

impl ProviderAdapter for OpenAiCompatibleProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn test_connection(&self) -> Result<ProviderConnectionTestResult, ProviderError> {
        build_provider_runtime()?.block_on(self.test_connection_async())
    }

    fn run(
        &self,
        request: &ModelRunRequest,
        cancellation: &RunCancellation,
        emit: &mut dyn FnMut(ModelRunEventEnvelope),
    ) -> Result<(), ProviderError> {
        build_provider_runtime()?.block_on(self.run_async(request, cancellation, emit))
    }
}

fn build_provider_runtime() -> Result<tokio::runtime::Runtime, ProviderError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| network_error("runtime_unavailable", false))
}

impl OpenAiCompatibleProvider {
    async fn test_connection_async(&self) -> Result<ProviderConnectionTestResult, ProviderError> {
        let credential = self.credential()?;
        let response = tokio::time::timeout(
            CONNECTION_TEST_TIMEOUT,
            self.client
                .get(&self.config.models_url)
                .bearer_auth(credential.as_str())
                .send(),
        )
        .await
        .map_err(|_| timeout_error("connection_test_timeout"))?
        .map_err(map_reqwest_error)?;
        drop(credential);

        if !response.status().is_success() {
            return Err(map_http_error(response.status(), response.headers()));
        }
        let payload = read_models_response(response).await?;
        let mut available_models = payload
            .data
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        available_models.sort();
        available_models.dedup();
        Ok(ProviderConnectionTestResult {
            provider_id: self.config.provider_id.clone(),
            authenticated: true,
            available_models,
            checked_at: now_timestamp(),
        })
    }

    async fn run_async(
        &self,
        request: &ModelRunRequest,
        cancellation: &RunCancellation,
        emit: &mut dyn FnMut(ModelRunEventEnvelope),
    ) -> Result<(), ProviderError> {
        if cancellation.is_cancelled() {
            emit_event(
                request,
                1,
                ModelRunEvent::Cancelled {
                    reason: RunCancelReason::UserRequested,
                    partial_content_retained: false,
                },
                emit,
            );
            return Ok(());
        }

        let credential = self.credential()?;
        let body = request_body(request, &self.config);
        let total_timeout = Duration::from_millis(request.budget.timeout_ms.max(1));
        let send = self
            .client
            .post(&self.config.base_url)
            .bearer_auth(credential.as_str())
            .json(&body)
            .send();
        let total_deadline = tokio::time::Instant::now() + total_timeout;
        let mut response = tokio::select! {
            biased;
            _ = wait_for_cancellation(cancellation) => {
                emit_event(request, 1, ModelRunEvent::Cancelled {
                    reason: RunCancelReason::UserRequested,
                    partial_content_retained: false,
                }, emit);
                return Ok(());
            }
            _ = tokio::time::sleep_until(total_deadline) => return Err(timeout_error("total_timeout")),
            result = send => result.map_err(map_reqwest_error)?,
        };
        drop(credential);

        if !response.status().is_success() {
            return Err(map_http_error(response.status(), response.headers()));
        }

        let mut sequence = 1;
        let mut retained_content = false;
        let mut received_reasoning_content = false;
        let mut decoder = SseDecoder::default();
        let mut final_reason = FinishReason::Unknown;
        let mut terminal_error = None;
        let mut latest_usage = ModelUsage::default();
        emit_event(request, sequence, ModelRunEvent::Started, emit);
        let first_token_deadline =
            tokio::time::Instant::now() + FIRST_TOKEN_TIMEOUT.min(total_timeout);
        let mut idle_deadline = first_token_deadline;
        let mut received_data_event = false;

        loop {
            let next = tokio::select! {
                biased;
                _ = wait_for_cancellation(cancellation) => {
                    emit_event(
                        request,
                        next_sequence(&mut sequence),
                        ModelRunEvent::Cancelled {
                            reason: RunCancelReason::UserRequested,
                            partial_content_retained: retained_content,
                        },
                        emit,
                    );
                    return Ok(());
                }
                _ = tokio::time::sleep_until(total_deadline) => return Err(timeout_error("total_timeout")),
                _ = tokio::time::sleep_until(idle_deadline) => return Err(timeout_error(if received_data_event {
                    "stream_idle_timeout"
                } else {
                    "first_token_timeout"
                })),
                result = response.chunk() => result,
            };
            let bytes = next
                .map_err(map_reqwest_error)?
                .ok_or_else(|| network_error("stream_ended_without_done", retained_content))?;
            let frames = decoder
                .push(&bytes)
                .map_err(|_| network_error("invalid_sse_utf8", retained_content))?;
            if !frames.is_empty() {
                received_data_event = true;
                idle_deadline =
                    tokio::time::Instant::now() + STREAM_IDLE_TIMEOUT.min(total_timeout);
            }
            for frame in frames {
                if frame.data == "[DONE]" {
                    let event = terminal_event(
                        terminal_error,
                        final_reason,
                        latest_usage,
                        retained_content,
                        received_reasoning_content,
                    );
                    emit_event(request, next_sequence(&mut sequence), event, emit);
                    return Ok(());
                }
                let chunk: ChatCompletionChunk = serde_json::from_str(&frame.data)
                    .map_err(|_| network_error("invalid_stream_event", retained_content))?;
                for choice in chunk.choices {
                    received_reasoning_content |= choice
                        .delta
                        .reasoning_content
                        .as_deref()
                        .is_some_and(|content| !content.is_empty());
                    if let Some(delta) = choice.delta.content
                        && !delta.is_empty()
                    {
                        retained_content = true;
                        emit_event(
                            request,
                            next_sequence(&mut sequence),
                            ModelRunEvent::TextDelta { delta },
                            emit,
                        );
                    }
                    if let Some(reason) = choice.finish_reason {
                        if reason == "insufficient_system_resource" {
                            terminal_error = Some(provider_error(
                                ProviderErrorCategory::Network,
                                "insufficient_system_resource",
                                "The provider could not complete the request due to insufficient capacity.",
                                true,
                                None,
                            ));
                        } else {
                            final_reason = map_finish_reason(&reason);
                        }
                    }
                }
                if let Some(usage) = chunk.usage {
                    latest_usage = usage.into();
                    emit_event(
                        request,
                        next_sequence(&mut sequence),
                        ModelRunEvent::UsageUpdated {
                            usage: latest_usage.clone(),
                        },
                        emit,
                    );
                }
            }
        }
    }
}

fn is_https_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| url.scheme() == "https" && url.host().is_some())
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ProviderModel>,
}

#[derive(Debug, Deserialize)]
struct ProviderModel {
    id: String,
}

async fn read_models_response(
    mut response: reqwest::Response,
) -> Result<ModelsResponse, ProviderError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MODELS_RESPONSE_BYTES as u64)
    {
        return Err(network_error("models_response_too_large", false));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
        if body.len().saturating_add(chunk.len()) > MAX_MODELS_RESPONSE_BYTES {
            return Err(network_error("models_response_too_large", false));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| network_error("invalid_models_response", false))
}

async fn wait_for_cancellation(cancellation: &RunCancellation) {
    while !cancellation.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    stream_options: StreamOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    mode: &'static str,
}

fn request_body(
    request: &ModelRunRequest,
    config: &OpenAiCompatibleConfig,
) -> ChatCompletionRequest {
    let mut messages = request
        .context_snapshot
        .selected_messages
        .iter()
        .map(|message| ChatMessage {
            role: match message.role {
                crate::domain::MessageRole::System => "system",
                crate::domain::MessageRole::Assistant => "assistant",
                crate::domain::MessageRole::User | crate::domain::MessageRole::Imported => "user",
            }
            .into(),
            content: blocks_plain_text(&message.content_blocks),
        })
        .collect::<Vec<_>>();
    messages.push(ChatMessage {
        role: "user".into(),
        content: request.context_snapshot.current_input.clone(),
    });
    ChatCompletionRequest {
        model: request.model_id.clone(),
        messages,
        stream: true,
        stream_options: StreamOptions {
            include_usage: true,
        },
        max_tokens: request.budget.max_output_tokens,
        thinking: config
            .disable_thinking
            .then_some(ThinkingConfig { mode: "disabled" }),
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[serde(default)]
    delta: ChatDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    prompt_cache_hit_tokens: Option<u64>,
}

impl From<OpenAiUsage> for ModelUsage {
    fn from(usage: OpenAiUsage) -> Self {
        Self {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cached_input_tokens: usage.prompt_cache_hit_tokens,
            cost_microunits: None,
        }
    }
}

fn next_sequence(sequence: &mut u64) -> u64 {
    *sequence += 1;
    *sequence
}

fn terminal_event(
    terminal_error: Option<ProviderError>,
    finish_reason: FinishReason,
    usage: ModelUsage,
    retained_content: bool,
    received_reasoning_content: bool,
) -> ModelRunEvent {
    if let Some(error) = terminal_error {
        return ModelRunEvent::Failed {
            error,
            partial_content_retained: retained_content,
        };
    }
    if retained_content {
        return ModelRunEvent::Completed {
            finish_reason,
            usage,
        };
    }

    let (code, safe_message) =
        if received_reasoning_content && finish_reason == FinishReason::Length {
            (
                "visible_response_exhausted",
                "The model reached the output limit before returning a visible answer.",
            )
        } else {
            (
                "empty_visible_response",
                "The provider completed without returning a visible answer.",
            )
        };
    ModelRunEvent::Failed {
        error: provider_error(
            ProviderErrorCategory::Unknown,
            code,
            safe_message,
            true,
            None,
        ),
        partial_content_retained: false,
    }
}

fn emit_event(
    request: &ModelRunRequest,
    sequence: u64,
    event: ModelRunEvent,
    emit: &mut dyn FnMut(ModelRunEventEnvelope),
) {
    emit(ModelRunEventEnvelope {
        contract_version: RUNTIME_CONTRACT_VERSION.into(),
        event_id: new_id("run-event"),
        run_id: request.run_id.clone(),
        node_id: request.node_id.clone(),
        sequence,
        occurred_at: now_timestamp(),
        event,
    });
}

fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentPolicy,
        "tool_calls" => FinishReason::ToolCall,
        _ => FinishReason::Unknown,
    }
}

fn map_credential_error(error: CredentialError) -> ProviderError {
    match error {
        CredentialError::NotFound => provider_error(
            ProviderErrorCategory::Authentication,
            "credential_not_found",
            "No credential is configured for this provider.",
            false,
            None,
        ),
        CredentialError::InvalidReference(_) | CredentialError::Unavailable => provider_error(
            ProviderErrorCategory::Authentication,
            "credential_unavailable",
            "The configured credential could not be accessed.",
            false,
            None,
        ),
    }
}

fn map_reqwest_error(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        timeout_error(if error.is_connect() {
            "connect_timeout"
        } else {
            "total_timeout"
        })
    } else if error.is_connect() {
        network_error("connect_failed", false)
    } else {
        network_error("request_failed", false)
    }
}

fn map_http_error(status: StatusCode, headers: &reqwest::header::HeaderMap) -> ProviderError {
    let retry_after_ms = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000));
    let (category, code, message, retryable) = match status.as_u16() {
        401 | 403 => (
            ProviderErrorCategory::Authentication,
            "authentication_failed",
            "The provider rejected the configured credential.",
            false,
        ),
        402 => (
            ProviderErrorCategory::InsufficientBalance,
            "insufficient_balance",
            "The provider account has insufficient balance.",
            false,
        ),
        404 => (
            ProviderErrorCategory::ModelUnavailable,
            "model_unavailable",
            "The selected model is unavailable.",
            false,
        ),
        429 => (
            ProviderErrorCategory::RateLimit,
            "rate_limited",
            "The provider is rate limiting requests.",
            true,
        ),
        400..=499 => (
            ProviderErrorCategory::InvalidRequest,
            "request_rejected",
            "The provider rejected the request.",
            false,
        ),
        _ => (
            ProviderErrorCategory::Network,
            "provider_unavailable",
            "The provider service is unavailable.",
            true,
        ),
    };
    provider_error(category, code, message, retryable, retry_after_ms).with_status(status.as_u16())
}

trait ProviderErrorStatus {
    fn with_status(self, status: u16) -> Self;
}

impl ProviderErrorStatus for ProviderError {
    fn with_status(mut self, status: u16) -> Self {
        self.provider_status = Some(status);
        self
    }
}

fn timeout_error(code: &str) -> ProviderError {
    provider_error(
        ProviderErrorCategory::Timeout,
        code,
        "The provider did not respond within the configured timeout.",
        false,
        None,
    )
}

fn network_error(code: &str, retryable: bool) -> ProviderError {
    provider_error(
        ProviderErrorCategory::Network,
        code,
        "The provider connection failed.",
        retryable,
        None,
    )
}

fn provider_error(
    category: ProviderErrorCategory,
    code: &str,
    safe_message: &str,
    retryable: bool,
    retry_after_ms: Option<u64>,
) -> ProviderError {
    ProviderError {
        category,
        provider_code: Some(code.into()),
        safe_message: safe_message.into(),
        retryable,
        retry_after_ms,
        provider_status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{CapabilityRequirement, ModelRunBudget};
    use crate::domain::{BranchType, ContextSnapshot};

    fn request() -> ModelRunRequest {
        ModelRunRequest {
            contract_version: RUNTIME_CONTRACT_VERSION.into(),
            run_id: "run-test".into(),
            conversation_id: "conversation-test".into(),
            node_id: "node-test".into(),
            context_snapshot: ContextSnapshot {
                id: "ctx-test".into(),
                conversation_id: "conversation-test".into(),
                parent_node_id: None,
                branch_type: BranchType::Continues,
                current_input: "当前问题".into(),
                selected_messages: vec![],
                selected_import_refs: vec![],
                explicit_constraints: vec![],
                omitted_messages: vec![],
                system_contract_version: "mindscape.context.v1".into(),
                estimated_tokens: 4,
                created_at: "2026-08-18T00:00:00Z".into(),
            },
            provider_id: "deepseek".into(),
            model_id: "deepseek-v4-flash".into(),
            capabilities: vec![CapabilityRequirement::TextInput],
            budget: ModelRunBudget {
                max_output_tokens: Some(256),
                max_cost_microunits: None,
                timeout_ms: 30_000,
            },
            idempotency_key: "idempotency-test".into(),
            created_at: "2026-08-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn builds_only_frozen_context_and_current_input() {
        let body = request_body(&request(), &OpenAiCompatibleConfig::deepseek());
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].content, "当前问题");
        assert_eq!(body.model, "deepseek-v4-flash");
        assert!(body.stream);
    }

    #[test]
    fn deepseek_requests_visible_answer_mode() {
        let body = request_body(&request(), &OpenAiCompatibleConfig::deepseek());
        let value = serde_json::to_value(body).expect("serialize request body");
        assert_eq!(value["thinking"]["type"], "disabled");
    }

    #[test]
    fn reasoning_only_output_limit_is_a_failed_run() {
        let event = terminal_event(
            None,
            FinishReason::Length,
            ModelUsage {
                output_tokens: Some(256),
                ..ModelUsage::default()
            },
            false,
            true,
        );
        let ModelRunEvent::Failed { error, .. } = event else {
            panic!("reasoning-only response must not complete successfully");
        };
        assert_eq!(
            error.provider_code.as_deref(),
            Some("visible_response_exhausted")
        );
        assert!(error.retryable);
    }

    #[test]
    fn visible_partial_output_can_complete_at_output_limit() {
        let event = terminal_event(
            None,
            FinishReason::Length,
            ModelUsage::default(),
            true,
            false,
        );
        assert!(matches!(
            event,
            ModelRunEvent::Completed {
                finish_reason: FinishReason::Length,
                ..
            }
        ));
    }

    #[test]
    fn maps_errors_without_exposing_provider_response_bodies() {
        let error = map_http_error(StatusCode::UNAUTHORIZED, &reqwest::header::HeaderMap::new());
        assert_eq!(error.category, ProviderErrorCategory::Authentication);
        assert_eq!(error.provider_status, Some(401));
        assert!(!error.safe_message.contains("Authorization"));

        let rate_limit = map_http_error(
            StatusCode::TOO_MANY_REQUESTS,
            &reqwest::header::HeaderMap::new(),
        );
        assert_eq!(rate_limit.category, ProviderErrorCategory::RateLimit);
        assert!(rate_limit.retryable);
    }

    #[test]
    fn rejects_non_https_provider_endpoints() {
        let config = OpenAiCompatibleConfig {
            base_url: "http://localhost/chat/completions".into(),
            ..OpenAiCompatibleConfig::deepseek()
        };
        let result = OpenAiCompatibleProvider::new(config, CredentialService::os_default());
        assert_eq!(
            result.unwrap_err().provider_code.as_deref(),
            Some("insecure_base_url")
        );
    }

    #[test]
    fn deepseek_descriptor_uses_verified_context_window() {
        let provider = OpenAiCompatibleProvider::new(
            OpenAiCompatibleConfig::deepseek(),
            CredentialService::os_default(),
        )
        .expect("create provider");
        assert_eq!(
            provider.descriptor.models["deepseek-v4-flash"].context_window_tokens,
            Some(1_000_000)
        );
    }

    #[test]
    fn rejects_non_https_connection_test_endpoint() {
        let config = OpenAiCompatibleConfig {
            models_url: "http://localhost/models".into(),
            ..OpenAiCompatibleConfig::deepseek()
        };
        let error = OpenAiCompatibleProvider::new(config, CredentialService::os_default())
            .expect_err("reject insecure models endpoint");
        assert_eq!(error.provider_code.as_deref(), Some("insecure_base_url"));
    }

    #[test]
    fn provider_runtime_enables_tokio_network_io() {
        build_provider_runtime()
            .expect("build provider runtime")
            .block_on(async {
                // Port zero is not a valid remote endpoint; the assertion is that Tokio returns
                // a normal I/O error instead of panicking because its I/O driver is disabled.
                let _ = tokio::net::TcpStream::connect(("127.0.0.1", 0)).await;
            });
    }
}
