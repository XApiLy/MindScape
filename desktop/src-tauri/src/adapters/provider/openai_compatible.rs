use std::{collections::HashMap, time::Duration};

use reqwest::{Client, StatusCode, redirect::Policy};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::CredentialService,
    domain::contracts::{
        EffectiveRunProfile, FinishReason, ModelRunEvent, ModelRunEventEnvelope, ModelRunRequest,
        ModelUsage, ProviderError, ProviderErrorCategory, RUNTIME_CONTRACT_VERSION, ReasoningMode,
        RunCancelReason,
    },
    domain::{CredentialError, CredentialRef, blocks_plain_text, new_id, now_timestamp},
};

use super::{
    GenerationParameterCapabilities, InputModality, ModelCapabilities, ParameterSupport,
    ProviderAdapter, ProviderConnectionTestResult, ProviderDescriptor, ProviderReasoningMode,
    ReasoningControl, RunCancellation, SseDecoder,
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
                        supports_reasoning: true,
                        reasoning_control: ReasoningControl::Effort,
                        reasoning_modes: vec![
                            ProviderReasoningMode::Disabled,
                            ProviderReasoningMode::High,
                            ProviderReasoningMode::Max,
                        ],
                        structured_output: true,
                        generation_parameters: GenerationParameterCapabilities {
                            max_output_tokens: ParameterSupport::Supported,
                            temperature: ParameterSupport::NonReasoningOnly,
                            top_p: ParameterSupport::NonReasoningOnly,
                            seed: ParameterSupport::Unsupported,
                        },
                        input_modalities: vec![InputModality::Text],
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
        let body = request_body(request, &self.config)?;
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
        let mut stream = OpenAiStreamAccumulator::default();
        let mut decoder = SseDecoder::default();
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
                        stream.cancelled_event(),
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
            let bytes = next.map_err(map_reqwest_error)?;
            let stream_ended = bytes.is_none();
            let frames = if let Some(bytes) = bytes {
                decoder
                    .push(&bytes)
                    .map_err(|_| network_error("invalid_sse_utf8", stream.retained_content))?
            } else {
                decoder
                    .finish()
                    .map_err(|_| network_error("invalid_sse_utf8", stream.retained_content))?
                    .into_iter()
                    .collect()
            };
            if !frames.is_empty() {
                received_data_event = true;
                idle_deadline =
                    tokio::time::Instant::now() + STREAM_IDLE_TIMEOUT.min(total_timeout);
            }
            for frame in frames {
                let frame_result = stream.apply_frame(&frame)?;
                for event in frame_result.events {
                    emit_event(request, next_sequence(&mut sequence), event, emit);
                }
                if let Some(event) = frame_result.terminal {
                    emit_event(request, next_sequence(&mut sequence), event, emit);
                    return Ok(());
                }
            }
            if stream_ended {
                let event = stream.finish_at_eof()?;
                emit_event(request, next_sequence(&mut sequence), event, emit);
                return Ok(());
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
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    mode: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct EffectiveDeepSeekParameters {
    thinking: ThinkingConfig,
    reasoning_effort: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<u64>,
    response_format: Option<serde_json::Value>,
}

fn map_effective_deepseek_profile(
    profile: &EffectiveRunProfile,
) -> Result<EffectiveDeepSeekParameters, ProviderError> {
    let reasoning_enabled = !matches!(profile.reasoning_mode, ReasoningMode::Off);
    if reasoning_enabled
        && (profile.generation_parameters.temperature.is_some()
            || profile.generation_parameters.top_p.is_some())
    {
        return Err(provider_error(
            ProviderErrorCategory::InvalidRequest,
            "unsupported_parameter_combination",
            "Temperature and top_p are not supported with DeepSeek thinking mode.",
            false,
            None,
        ));
    }
    if profile.generation_parameters.seed.is_some() {
        return Err(provider_error(
            ProviderErrorCategory::InvalidRequest,
            "unsupported_parameter",
            "The selected DeepSeek model does not support seed.",
            false,
            None,
        ));
    }
    if profile.reasoning_budget.is_some() {
        return Err(provider_error(
            ProviderErrorCategory::InvalidRequest,
            "unsupported_reasoning_budget",
            "The selected DeepSeek model uses reasoning effort instead of a token budget.",
            false,
            None,
        ));
    }
    let reasoning_effort = match profile.reasoning_mode {
        ReasoningMode::Off => None,
        ReasoningMode::Standard => Some("high".into()),
        ReasoningMode::Deep => Some("max".into()),
        ReasoningMode::Custom => Some(
            profile
                .generation_parameters
                .vendor_parameters
                .get("reasoning_effort")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
                .filter(|value| matches!(value.as_str(), "high" | "max"))
                .ok_or_else(|| {
                    provider_error(
                        ProviderErrorCategory::InvalidRequest,
                        "invalid_reasoning_effort",
                        "Custom DeepSeek reasoning requires reasoning_effort high or max.",
                        false,
                        None,
                    )
                })?,
        ),
    };
    let response_format = match profile.generation_parameters.response_format.as_deref() {
        None => None,
        Some("json_object") => Some(serde_json::json!({"type": "json_object"})),
        Some(_) => {
            return Err(provider_error(
                ProviderErrorCategory::InvalidRequest,
                "unsupported_response_format",
                "The selected DeepSeek model only supports json_object structured output.",
                false,
                None,
            ));
        }
    };
    Ok(EffectiveDeepSeekParameters {
        thinking: ThinkingConfig {
            mode: if reasoning_enabled {
                "enabled"
            } else {
                "disabled"
            },
        },
        reasoning_effort,
        temperature: profile.generation_parameters.temperature,
        top_p: profile.generation_parameters.top_p,
        max_tokens: profile
            .generation_parameters
            .max_output_tokens
            .or(profile.budget_envelope.max_output_tokens),
        response_format,
    })
}

fn request_body(
    request: &ModelRunRequest,
    config: &OpenAiCompatibleConfig,
) -> Result<ChatCompletionRequest, ProviderError> {
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
    let effective = request
        .effective_run_profile
        .as_ref()
        .map(|profile| {
            if profile.contract_version
                != crate::domain::contracts::EFFECTIVE_RUN_PROFILE_CONTRACT_VERSION
            {
                return Err(provider_error(
                    ProviderErrorCategory::InvalidRequest,
                    "unsupported_run_profile_contract",
                    "The effective run profile contract is not supported.",
                    false,
                    None,
                ));
            }
            if profile.provider_id != request.provider_id || profile.model_id != request.model_id {
                return Err(provider_error(
                    ProviderErrorCategory::InvalidRequest,
                    "run_profile_target_mismatch",
                    "The effective run profile does not match the selected provider and model.",
                    false,
                    None,
                ));
            }
            map_effective_deepseek_profile(profile)
        })
        .transpose()?;
    Ok(ChatCompletionRequest {
        model: request.model_id.clone(),
        messages,
        stream: true,
        stream_options: StreamOptions {
            include_usage: true,
        },
        max_tokens: effective
            .as_ref()
            .and_then(|value| value.max_tokens)
            .or(request.budget.max_output_tokens),
        thinking: effective
            .as_ref()
            .map(|value| value.thinking.clone())
            .or_else(|| {
                config
                    .disable_thinking
                    .then_some(ThinkingConfig { mode: "disabled" })
            }),
        reasoning_effort: effective
            .as_ref()
            .and_then(|value| value.reasoning_effort.clone()),
        temperature: effective.as_ref().and_then(|value| value.temperature),
        top_p: effective.as_ref().and_then(|value| value.top_p),
        response_format: effective.and_then(|value| value.response_format),
    })
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

#[derive(Debug)]
struct StreamFrameResult {
    events: Vec<ModelRunEvent>,
    terminal: Option<ModelRunEvent>,
}

#[derive(Debug)]
struct OpenAiStreamAccumulator {
    retained_content: bool,
    received_reasoning_content: bool,
    final_reason: FinishReason,
    terminal_error: Option<ProviderError>,
    latest_usage: ModelUsage,
}

impl Default for OpenAiStreamAccumulator {
    fn default() -> Self {
        Self {
            retained_content: false,
            received_reasoning_content: false,
            final_reason: FinishReason::Unknown,
            terminal_error: None,
            latest_usage: ModelUsage::default(),
        }
    }
}

impl OpenAiStreamAccumulator {
    fn cancelled_event(&self) -> ModelRunEvent {
        ModelRunEvent::Cancelled {
            reason: RunCancelReason::UserRequested,
            partial_content_retained: self.retained_content,
        }
    }

    fn apply_frame(&mut self, frame: &super::SseFrame) -> Result<StreamFrameResult, ProviderError> {
        if frame.data == "[DONE]" {
            return Ok(StreamFrameResult {
                events: Vec::new(),
                terminal: Some(self.terminal_event()),
            });
        }
        let chunk: ChatCompletionChunk = serde_json::from_str(&frame.data)
            .map_err(|_| network_error("invalid_stream_event", self.retained_content))?;
        let mut events = Vec::new();
        for choice in chunk.choices {
            self.received_reasoning_content |= choice
                .delta
                .reasoning_content
                .as_deref()
                .is_some_and(|content| !content.is_empty());
            if let Some(delta) = choice.delta.content
                && !delta.is_empty()
            {
                self.retained_content = true;
                events.push(ModelRunEvent::TextDelta { delta });
            }
            if let Some(reason) = choice.finish_reason {
                if reason == "insufficient_system_resource" {
                    self.terminal_error = Some(provider_error(
                        ProviderErrorCategory::Network,
                        "insufficient_system_resource",
                        "The provider could not complete the request due to insufficient capacity.",
                        true,
                        None,
                    ));
                } else {
                    self.final_reason = map_finish_reason(&reason);
                }
            }
        }
        if let Some(usage) = chunk.usage {
            self.latest_usage = usage.into();
            events.push(ModelRunEvent::UsageUpdated {
                usage: self.latest_usage.clone(),
            });
        }
        Ok(StreamFrameResult {
            events,
            terminal: None,
        })
    }

    fn finish_at_eof(&mut self) -> Result<ModelRunEvent, ProviderError> {
        if self.final_reason == FinishReason::Unknown && self.terminal_error.is_none() {
            return Err(network_error(
                "stream_ended_without_done",
                self.retained_content,
            ));
        }
        Ok(self.terminal_event())
    }

    fn terminal_event(&mut self) -> ModelRunEvent {
        terminal_event(
            self.terminal_error.take(),
            self.final_reason,
            std::mem::take(&mut self.latest_usage),
            self.retained_content,
            self.received_reasoning_content,
        )
    }
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
    use crate::domain::contracts::{
        CapabilityRequirement, EffectiveRunProfile, GenerationParameters, ModelRunBudget,
        RunBudgetEnvelope, RunCapabilitySnapshot, RunValueOrigin, ToolPermission,
    };
    use crate::domain::{BranchType, ContextSnapshot};
    use std::collections::BTreeMap;

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
            effective_run_profile: None,
            idempotency_key: "idempotency-test".into(),
            created_at: "2026-08-18T00:00:00Z".into(),
        }
    }

    fn profile(mode: ReasoningMode) -> EffectiveRunProfile {
        EffectiveRunProfile {
            contract_version: "mindscape.effective-run-profile.v1".into(),
            provider_id: "deepseek".into(),
            model_id: "deepseek-v4-flash".into(),
            reasoning_mode: mode,
            reasoning_budget: None,
            generation_parameters: GenerationParameters {
                temperature: None,
                top_p: None,
                max_output_tokens: Some(512),
                response_format: None,
                seed: None,
                vendor_parameters: BTreeMap::new(),
            },
            context_policy: "focused".into(),
            allowed_capabilities: vec![CapabilityRequirement::TextInput],
            tool_permission: ToolPermission::Disabled,
            budget_envelope: RunBudgetEnvelope {
                max_input_tokens: None,
                max_reasoning_tokens: None,
                max_output_tokens: Some(1024),
                max_cost_microunits: None,
                timeout_ms: 30_000,
            },
            value_origins: BTreeMap::<String, RunValueOrigin>::new(),
            capability_snapshot: RunCapabilitySnapshot {
                catalog_version: "m2".into(),
                context_window_tokens: Some(1_000_000),
                supported_capabilities: vec![CapabilityRequirement::TextInput],
                unsupported_parameters: vec![],
            },
        }
    }

    fn decode_stream_fixture(
        fixture: &[u8],
        network_chunk_size: usize,
    ) -> Result<Vec<ModelRunEvent>, ProviderError> {
        let mut decoder = SseDecoder::default();
        let mut stream = OpenAiStreamAccumulator::default();
        let mut events = Vec::new();
        for chunk in fixture.chunks(network_chunk_size.max(1)) {
            let frames = decoder
                .push(chunk)
                .map_err(|_| network_error("invalid_sse_utf8", stream.retained_content))?;
            for frame in frames {
                let result = stream.apply_frame(&frame)?;
                events.extend(result.events);
                if let Some(terminal) = result.terminal {
                    events.push(terminal);
                    return Ok(events);
                }
            }
        }
        if let Some(frame) = decoder
            .finish()
            .map_err(|_| network_error("invalid_sse_utf8", stream.retained_content))?
        {
            let result = stream.apply_frame(&frame)?;
            events.extend(result.events);
            if let Some(terminal) = result.terminal {
                events.push(terminal);
                return Ok(events);
            }
        }
        events.push(stream.finish_at_eof()?);
        Ok(events)
    }

    #[test]
    fn maps_deepseek_reasoning_modes_deterministically() {
        assert_eq!(
            map_effective_deepseek_profile(&profile(ReasoningMode::Off))
                .unwrap()
                .thinking
                .mode,
            "disabled"
        );
        assert_eq!(
            map_effective_deepseek_profile(&profile(ReasoningMode::Standard))
                .unwrap()
                .reasoning_effort
                .as_deref(),
            Some("high")
        );
        assert_eq!(
            map_effective_deepseek_profile(&profile(ReasoningMode::Deep))
                .unwrap()
                .reasoning_effort
                .as_deref(),
            Some("max")
        );
    }

    #[test]
    fn rejects_thinking_sampling_and_reasoning_token_budget() {
        let mut sampling = profile(ReasoningMode::Standard);
        sampling.generation_parameters.temperature = Some(0.7);
        assert_eq!(
            map_effective_deepseek_profile(&sampling)
                .unwrap_err()
                .provider_code
                .as_deref(),
            Some("unsupported_parameter_combination")
        );
        let mut budget = profile(ReasoningMode::Deep);
        budget.reasoning_budget = Some(256);
        assert_eq!(
            map_effective_deepseek_profile(&budget)
                .unwrap_err()
                .provider_code
                .as_deref(),
            Some("unsupported_reasoning_budget")
        );
    }

    #[test]
    fn effective_profile_is_serialized_into_the_provider_request() {
        let mut request = request();
        let mut effective = profile(ReasoningMode::Deep);
        effective.generation_parameters.response_format = Some("json_object".into());
        request.effective_run_profile = Some(effective);
        let body = request_body(&request, &OpenAiCompatibleConfig::deepseek()).unwrap();
        let value = serde_json::to_value(body).expect("serialize request body");
        assert_eq!(value["thinking"]["type"], "enabled");
        assert_eq!(value["reasoning_effort"], "max");
        assert_eq!(value["max_tokens"], 512);
        assert_eq!(value["response_format"]["type"], "json_object");
    }

    #[test]
    fn rejects_a_profile_for_a_different_runtime_target() {
        let mut request = request();
        let mut effective = profile(ReasoningMode::Off);
        effective.model_id = "different-model".into();
        request.effective_run_profile = Some(effective);
        assert_eq!(
            request_body(&request, &OpenAiCompatibleConfig::deepseek())
                .unwrap_err()
                .provider_code
                .as_deref(),
            Some("run_profile_target_mismatch")
        );
    }

    #[test]
    fn builds_only_frozen_context_and_current_input() {
        let body = request_body(&request(), &OpenAiCompatibleConfig::deepseek()).unwrap();
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].content, "当前问题");
        assert_eq!(body.model, "deepseek-v4-flash");
        assert!(body.stream);
    }

    #[test]
    fn deepseek_requests_visible_answer_mode() {
        let body = request_body(&request(), &OpenAiCompatibleConfig::deepseek()).unwrap();
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
    fn reasoning_only_stop_is_failed_as_empty_visible_response() {
        let ModelRunEvent::Failed {
            error,
            partial_content_retained,
        } = terminal_event(None, FinishReason::Stop, ModelUsage::default(), false, true)
        else {
            panic!("reasoning-only stop must fail");
        };
        assert!(!partial_content_retained);
        assert_eq!(
            error.provider_code.as_deref(),
            Some("empty_visible_response")
        );
    }

    #[test]
    fn reasoning_with_visible_content_completes_and_retains_usage() {
        let usage = ModelUsage {
            input_tokens: Some(12),
            output_tokens: Some(34),
            cached_input_tokens: Some(3),
            cost_microunits: None,
        };
        let ModelRunEvent::Completed {
            finish_reason,
            usage: retained,
        } = terminal_event(None, FinishReason::Stop, usage.clone(), true, true)
        else {
            panic!("visible content must complete");
        };
        assert_eq!(finish_reason, FinishReason::Stop);
        assert_eq!(retained, usage);
    }

    #[test]
    fn provider_terminal_error_wins_over_partial_content() {
        let provider_failure = provider_error(
            ProviderErrorCategory::Network,
            "insufficient_system_resource",
            "The provider could not complete the request due to insufficient capacity.",
            true,
            None,
        );
        let ModelRunEvent::Failed {
            error,
            partial_content_retained,
        } = terminal_event(
            Some(provider_failure),
            FinishReason::Stop,
            ModelUsage::default(),
            true,
            false,
        )
        else {
            panic!("provider error must be terminal");
        };
        assert_eq!(
            error.provider_code.as_deref(),
            Some("insufficient_system_resource")
        );
        assert!(partial_content_retained);
    }

    #[test]
    fn decodes_deepseek_reasoning_content_and_usage_fixture_without_merging_channels() {
        let reasoning: ChatCompletionChunk = serde_json::from_str(
            r#"{"choices":[{"delta":{"reasoning_content":"内部推理"},"finish_reason":null}],"usage":null}"#,
        )
        .expect("reasoning fixture");
        assert_eq!(
            reasoning.choices[0].delta.reasoning_content.as_deref(),
            Some("内部推理")
        );
        assert!(reasoning.choices[0].delta.content.is_none());

        let visible: ChatCompletionChunk = serde_json::from_str(
            r#"{"choices":[{"delta":{"content":"可见答案"},"finish_reason":"stop"}],"usage":null}"#,
        )
        .expect("visible fixture");
        assert_eq!(
            visible.choices[0].delta.content.as_deref(),
            Some("可见答案")
        );
        assert!(visible.choices[0].delta.reasoning_content.is_none());

        let usage: ChatCompletionChunk = serde_json::from_str(
            r#"{"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":11,"prompt_cache_hit_tokens":2}}"#,
        )
        .expect("usage fixture");
        assert_eq!(
            ModelUsage::from(usage.usage.expect("usage")),
            ModelUsage {
                input_tokens: Some(7),
                output_tokens: Some(11),
                cached_input_tokens: Some(2),
                cost_microunits: None,
            }
        );
    }

    #[test]
    fn streamed_markdown_fixture_preserves_unclosed_source_and_flushes_terminal_usage() {
        let events =
            decode_stream_fixture(include_bytes!("fixtures/deepseek_markdown_unclosed.sse"), 7)
                .expect("decode markdown stream fixture");
        let raw_markdown = events
            .iter()
            .filter_map(|event| match event {
                ModelRunEvent::TextDelta { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(
            raw_markdown,
            "# 流式标题\n\n| 列 | 值 |\n| --- | --- |\n| A | 1 |\n\n```rust\nfn main() {"
        );
        assert!(!raw_markdown.ends_with("```"));
        assert!(matches!(
            events.last(),
            Some(ModelRunEvent::Completed {
                finish_reason: FinishReason::Stop,
                usage: ModelUsage {
                    input_tokens: Some(21),
                    output_tokens: Some(34),
                    cached_input_tokens: Some(5),
                    ..
                }
            })
        ));
    }

    #[test]
    fn reasoning_only_fixture_keeps_usage_event_and_fails_visible_answer() {
        let events = decode_stream_fixture(
            include_bytes!("fixtures/deepseek_reasoning_only_length.sse"),
            5,
        )
        .expect("decode reasoning-only fixture");
        assert!(events.iter().any(|event| matches!(
            event,
            ModelRunEvent::UsageUpdated {
                usage: ModelUsage {
                    output_tokens: Some(256),
                    ..
                }
            }
        )));
        assert!(matches!(
            events.last(),
            Some(ModelRunEvent::Failed { error, partial_content_retained: false })
                if error.provider_code.as_deref() == Some("visible_response_exhausted")
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ModelRunEvent::TextDelta { .. }))
        );
    }

    #[test]
    fn empty_visible_fixture_is_failed_without_synthesizing_markdown() {
        let events =
            decode_stream_fixture(include_bytes!("fixtures/deepseek_empty_visible.sse"), 3)
                .expect("decode empty visible fixture");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.last(),
            Some(ModelRunEvent::Failed { error, partial_content_retained: false })
                if error.provider_code.as_deref() == Some("empty_visible_response")
        ));
    }

    #[test]
    fn provider_failure_fixture_retains_partial_unclosed_markdown() {
        let events = decode_stream_fixture(
            include_bytes!("fixtures/deepseek_partial_provider_failure.sse"),
            8,
        )
        .expect("decode provider failure fixture");
        assert!(matches!(
            events.as_slice(),
            [
                ModelRunEvent::TextDelta { delta },
                ModelRunEvent::Failed {
                    error,
                    partial_content_retained: true
                }
            ] if delta == "> 未完成引用\n\n```json\n{\"partial\":"
                && error.provider_code.as_deref() == Some("insufficient_system_resource")
        ));
    }

    #[test]
    fn cancellation_after_partial_markdown_retains_raw_content() {
        let frame = crate::adapters::provider::SseFrame {
            event: None,
            data: r#"{"choices":[{"delta":{"content":"```text\n未完成"},"finish_reason":null}],"usage":null}"#.into(),
            id: None,
            retry_ms: None,
        };
        let mut stream = OpenAiStreamAccumulator::default();
        let result = stream.apply_frame(&frame).expect("decode partial markdown");
        assert!(matches!(
            result.events.as_slice(),
            [ModelRunEvent::TextDelta { delta }] if delta == "```text\n未完成"
        ));
        assert!(matches!(
            stream.cancelled_event(),
            ModelRunEvent::Cancelled {
                reason: RunCancelReason::UserRequested,
                partial_content_retained: true
            }
        ));
    }

    #[test]
    fn eof_with_finish_reason_completes_without_done_sentinel() {
        let fixture = r#"data: {"choices":[{"delta":{"content":"结尾"},"finish_reason":"stop"}],"usage":null}"#
            .as_bytes();
        let events = decode_stream_fixture(fixture, 4).expect("flush final frame at eof");
        assert!(matches!(
            events.as_slice(),
            [ModelRunEvent::TextDelta { delta }, ModelRunEvent::Completed { finish_reason: FinishReason::Stop, .. }]
                if delta == "结尾"
        ));
    }

    #[test]
    fn eof_without_finish_reason_fails_and_marks_partial_content_retryable() {
        let fixture = r#"data: {"choices":[{"delta":{"content":"未完成"},"finish_reason":null}],"usage":null}"#
            .as_bytes();
        let error = decode_stream_fixture(fixture, 6).expect_err("reject truncated stream");
        assert_eq!(
            error.provider_code.as_deref(),
            Some("stream_ended_without_done")
        );
        assert!(error.retryable);
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
    fn deepseek_descriptor_declares_reasoning_controls_without_silent_sampling() {
        let provider = OpenAiCompatibleProvider::new(
            OpenAiCompatibleConfig::deepseek(),
            CredentialService::os_default(),
        )
        .expect("create provider");
        let capabilities = &provider.descriptor.models["deepseek-v4-flash"];
        assert_eq!(capabilities.reasoning_control, ReasoningControl::Effort);
        assert_eq!(
            capabilities.generation_parameters.temperature,
            ParameterSupport::NonReasoningOnly
        );
    }

    #[test]
    fn deepseek_capability_snapshot_serializes_for_frontend_contract() {
        let provider = OpenAiCompatibleProvider::new(
            OpenAiCompatibleConfig::deepseek(),
            CredentialService::os_default(),
        )
        .expect("create provider");
        let value = serde_json::to_value(&provider.descriptor.models["deepseek-v4-flash"])
            .expect("serialize capabilities");
        assert_eq!(value["reasoningControl"], "effort");
        assert_eq!(
            value["generationParameters"]["temperature"],
            "nonReasoningOnly"
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
