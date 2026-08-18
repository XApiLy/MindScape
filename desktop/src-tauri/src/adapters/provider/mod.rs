mod mock;
mod openai_compatible;
mod registry;
mod sse;

#[allow(unused_imports)]
pub use mock::{MockProvider, MockScenario};
pub use openai_compatible::{OpenAiCompatibleConfig, OpenAiCompatibleProvider};
#[allow(unused_imports)]
pub use registry::{
    ModelCapabilities, ProviderAdapter, ProviderConnectionTestResult, ProviderDescriptor,
    ProviderRegistry, ProviderRuntime, ProviderRuntimeError, RunCancellation,
};
#[allow(unused_imports)]
pub use sse::{SseDecoder, SseFrame};

#[cfg(test)]
mod tests {
    use crate::domain::contracts::{
        CapabilityRequirement, ModelRunBudget, ModelRunEvent, ModelRunRequest, ProviderError,
        ProviderErrorCategory, RUNTIME_CONTRACT_VERSION,
    };
    use crate::domain::{BranchType, ContextSnapshot};

    use super::*;

    fn request(capabilities: Vec<CapabilityRequirement>) -> ModelRunRequest {
        ModelRunRequest {
            contract_version: RUNTIME_CONTRACT_VERSION.into(),
            run_id: "run-1".into(),
            conversation_id: "conversation-1".into(),
            node_id: "node-1".into(),
            context_snapshot: ContextSnapshot {
                id: "ctx-1".into(),
                conversation_id: "conversation-1".into(),
                parent_node_id: None,
                branch_type: BranchType::Continues,
                current_input: "hello".into(),
                selected_messages: vec![],
                selected_import_refs: vec![],
                explicit_constraints: vec![],
                omitted_messages: vec![],
                system_contract_version: "mindscape.context.v1".into(),
                estimated_tokens: 2,
                created_at: "2026-08-14T00:00:00Z".into(),
            },
            provider_id: "mock".into(),
            model_id: "mock-stream-v1".into(),
            capabilities,
            budget: ModelRunBudget {
                max_output_tokens: Some(128),
                max_cost_microunits: None,
                timeout_ms: 5_000,
            },
            idempotency_key: "idempotency-1".into(),
            created_at: "2026-08-14T00:00:00Z".into(),
        }
    }

    #[test]
    fn routes_mock_events_with_monotonic_envelopes() {
        let mut registry = ProviderRegistry::default();
        registry.register(MockProvider::standard()).unwrap();
        let runtime = ProviderRuntime::new(registry);
        let mut events = Vec::new();
        runtime
            .run(
                &request(vec![CapabilityRequirement::TextInput]),
                &RunCancellation::default(),
                &mut |event| events.push(event),
            )
            .unwrap();

        assert!(matches!(events[0].event, ModelRunEvent::Started));
        assert!(matches!(
            events.last().unwrap().event,
            ModelRunEvent::Completed { .. }
        ));
        assert_eq!(
            events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            (1..=events.len() as u64).collect::<Vec<_>>()
        );
        assert!(events.iter().all(|event| event.run_id == "run-1"));
    }

    #[test]
    fn rejects_unsupported_capabilities_before_execution() {
        let mut registry = ProviderRegistry::default();
        registry.register(MockProvider::standard()).unwrap();
        let error = ProviderRuntime::new(registry)
            .run(
                &request(vec![CapabilityRequirement::ImageInput]),
                &RunCancellation::default(),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProviderRuntimeError::CapabilityUnsupported { .. }
        ));
    }

    #[test]
    fn mock_failure_is_an_event_not_assistant_text() {
        let error = ProviderError {
            category: ProviderErrorCategory::RateLimit,
            provider_code: Some("rate_limit".into()),
            safe_message: "Try again later.".into(),
            retryable: true,
            retry_after_ms: Some(1_000),
            provider_status: Some(429),
        };
        let mut registry = ProviderRegistry::default();
        registry
            .register(MockProvider::new(MockScenario::Fail {
                after_chunks: 1,
                error,
            }))
            .unwrap();
        let mut events = Vec::new();
        ProviderRuntime::new(registry)
            .run(
                &request(vec![]),
                &RunCancellation::default(),
                &mut |event| events.push(event),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            ModelRunEvent::Failed { .. }
        ));
    }

    #[test]
    fn cancelled_mock_run_emits_cancelled_without_completion() {
        let cancellation = RunCancellation::default();
        cancellation.cancel();
        let mut registry = ProviderRegistry::default();
        registry.register(MockProvider::standard()).unwrap();
        let mut events = Vec::new();
        ProviderRuntime::new(registry)
            .run(&request(vec![]), &cancellation, &mut |event| {
                events.push(event)
            })
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            ModelRunEvent::Cancelled { .. }
        ));
    }

    #[test]
    fn connection_test_routes_to_registered_provider() {
        let mut registry = ProviderRegistry::default();
        registry.register(MockProvider::standard()).unwrap();
        let result = ProviderRuntime::new(registry)
            .test_connection("mock")
            .expect("test mock connection");
        assert!(result.authenticated);
        assert_eq!(result.provider_id, "mock");
        assert_eq!(result.available_models, vec!["mock-stream-v1"]);
    }
}
