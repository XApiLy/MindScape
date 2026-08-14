use std::{collections::HashMap, thread, time::Duration};

use crate::domain::contracts::{
    FinishReason, ModelRunEvent, ModelRunEventEnvelope, ModelRunRequest, ModelUsage, ProviderError,
    RUNTIME_CONTRACT_VERSION, RunCancelReason,
};
use crate::domain::{new_id, now_timestamp};

use super::{ModelCapabilities, ProviderAdapter, ProviderDescriptor, RunCancellation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockScenario {
    Success {
        chunks: Vec<String>,
        delay_ms: u64,
    },
    Fail {
        after_chunks: usize,
        error: ProviderError,
    },
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    descriptor: ProviderDescriptor,
    scenario: MockScenario,
}

impl MockProvider {
    pub fn new(scenario: MockScenario) -> Self {
        let mut models = HashMap::new();
        models.insert(
            "mock-stream-v1".into(),
            ModelCapabilities {
                text_input: true,
                image_input: false,
                tool_calling: false,
                usage_reporting: true,
                streaming: true,
                context_window_tokens: Some(16_384),
            },
        );
        Self {
            descriptor: ProviderDescriptor {
                id: "mock".into(),
                display_name: "MindScape Mock Provider".into(),
                default_base_url: None,
                custom_base_url_allowed: false,
                credential_required: false,
                models,
            },
            scenario,
        }
    }

    pub fn standard() -> Self {
        Self::new(MockScenario::Success {
            chunks: vec![
                "这是 Rust ProviderRuntime 返回的".into(),
                "统一流式事件。".into(),
                "事件会先持久化，".into(),
                "再通过 Tauri Channel 发送到 Chat。".into(),
            ],
            delay_ms: 80,
        })
    }
}

impl ProviderAdapter for MockProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn run(
        &self,
        request: &ModelRunRequest,
        cancellation: &RunCancellation,
        emit: &mut dyn FnMut(ModelRunEventEnvelope),
    ) -> Result<(), ProviderError> {
        let mut sequence = 0;
        emit_envelope(request, &mut sequence, ModelRunEvent::Started, emit);

        let (chunks, delay_ms, failure) = match &self.scenario {
            MockScenario::Success { chunks, delay_ms } => (chunks.clone(), *delay_ms, None),
            MockScenario::Fail {
                after_chunks,
                error,
            } => (
                vec!["partial".to_string(); *after_chunks],
                0,
                Some(error.clone()),
            ),
        };

        for chunk in chunks {
            if cancellation.is_cancelled() {
                let partial_content_retained = sequence > 1;
                emit_envelope(
                    request,
                    &mut sequence,
                    ModelRunEvent::Cancelled {
                        reason: RunCancelReason::UserRequested,
                        partial_content_retained,
                    },
                    emit,
                );
                return Ok(());
            }
            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }
            emit_envelope(
                request,
                &mut sequence,
                ModelRunEvent::TextDelta { delta: chunk },
                emit,
            );
        }

        if let Some(error) = failure {
            let partial_content_retained = sequence > 1;
            emit_envelope(
                request,
                &mut sequence,
                ModelRunEvent::Failed {
                    error,
                    partial_content_retained,
                },
                emit,
            );
            return Ok(());
        }

        emit_envelope(
            request,
            &mut sequence,
            ModelRunEvent::Completed {
                finish_reason: FinishReason::Stop,
                usage: ModelUsage::default(),
            },
            emit,
        );
        Ok(())
    }
}

fn emit_envelope(
    request: &ModelRunRequest,
    sequence: &mut u64,
    event: ModelRunEvent,
    emit: &mut dyn FnMut(ModelRunEventEnvelope),
) {
    *sequence += 1;
    emit(ModelRunEventEnvelope {
        contract_version: RUNTIME_CONTRACT_VERSION.into(),
        event_id: new_id("run-event"),
        run_id: request.run_id.clone(),
        node_id: request.node_id.clone(),
        sequence: *sequence,
        occurred_at: now_timestamp(),
        event,
    });
}
