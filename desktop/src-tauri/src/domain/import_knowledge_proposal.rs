use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::contracts::{
    EvidenceRef, EvidenceTarget, GeneratorKind, GeneratorRef, ImportRevision, ImportRevisionStatus,
    ImportSource, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntity, KnowledgeEntityKind, KnowledgeScope,
    KnowledgeStatus, ScopedEvidenceRef,
};

pub const IMPORT_KNOWLEDGE_PROPOSAL_CONTRACT_VERSION: &str =
    "mindscape.import-knowledge-proposal.v1";
const MAX_PROPOSALS_PER_REQUEST: usize = 64;
const MAX_ALIASES_PER_PROPOSAL: usize = 16;
const MAX_EVIDENCE_PER_PROPOSAL: usize = 16;
const MAX_NAME_CHARS: usize = 240;
const MAX_ALIAS_CHARS: usize = 120;

/// Explicit user request to analyze an immutable import revision.
///
/// This command is separate from import ingestion: importing content never
/// triggers generative analysis automatically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportKnowledgeProposalRequestInput {
    pub request_id: String,
    pub import_source_id: String,
    pub import_revision_id: String,
    pub expected_source_content_hash: String,
    pub selected_message_ids: Vec<String>,
    pub target_scope: KnowledgeScope,
    pub requested_at: String,
}

/// Kernel-authored context used to verify a user-selected target scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportKnowledgeProposalTargetContext {
    pub workspace_id: String,
    pub conversation_id: String,
    pub active_focus_frame_id: Option<String>,
}

/// Authoritative source slice built from a stored ImportedMessage.
///
/// The provider receives the stable message ID, but it does not author the
/// locator, content hash, excerpt, or EvidenceRef identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportKnowledgeSourceSnapshot {
    pub imported_message_id: String,
    pub import_revision_id: String,
    pub source_locator: String,
    pub content_hash: String,
    pub excerpt: String,
}

/// Untrusted structured output from the extraction provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportKnowledgeSuggestionDraft {
    pub ordinal: u32,
    pub kind: KnowledgeEntityKind,
    pub name: String,
    pub aliases: Vec<String>,
    pub evidence_message_ids: Vec<String>,
}

/// Immutable suggestion awaiting an explicit user decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportKnowledgeEntityProposal {
    pub contract_version: String,
    pub proposal_id: String,
    pub request_id: String,
    pub import_source_id: String,
    pub import_revision_id: String,
    pub conversation_id: String,
    pub suggested_kind: KnowledgeEntityKind,
    pub suggested_name: String,
    pub suggested_aliases: Vec<String>,
    pub target_scope: KnowledgeScope,
    pub evidence: Vec<EvidenceRef>,
    pub generator: GeneratorRef,
    pub proposal_revision: u64,
    pub proposed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportKnowledgeProposalBatchProjection {
    pub contract_version: String,
    pub request_id: String,
    pub import_source_id: String,
    pub import_revision_id: String,
    pub conversation_id: String,
    pub source_content_hash: String,
    pub target_scope: KnowledgeScope,
    pub generation_run_id: String,
    pub generator: GeneratorRef,
    pub proposals: Vec<ImportKnowledgeEntityProposal>,
    pub batch_revision: u64,
    pub requested_at: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportKnowledgeProposalReviewAction {
    Confirm,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportKnowledgeProposalReviewChoice {
    Confirm {
        kind: KnowledgeEntityKind,
        name: String,
        aliases: Vec<String>,
    },
    Reject {
        reason: Option<String>,
    },
}

/// User-authored decision. Entity identity and evidence are never accepted
/// from the frontend; the application supplies a new entity ID only for the
/// first successful confirm plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportKnowledgeProposalReviewCommandInput {
    pub decision_id: String,
    pub request_id: String,
    pub proposal_id: String,
    pub expected_proposal_revision: u64,
    pub choice: ImportKnowledgeProposalReviewChoice,
    pub decided_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportKnowledgeProposalReviewProjection {
    pub contract_version: String,
    pub decision_id: String,
    pub request_id: String,
    pub proposal_id: String,
    pub proposal_revision: u64,
    pub action: ImportKnowledgeProposalReviewAction,
    pub entity_id: Option<String>,
    pub entity_status: Option<KnowledgeStatus>,
    pub decided_by: GeneratorRef,
    pub decided_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportKnowledgeProposalReviewPlan {
    pub decision: ImportKnowledgeProposalReviewProjection,
    pub entity: Option<KnowledgeEntity>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportKnowledgeProposalError {
    #[error("import knowledge proposal field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("import knowledge proposal request requires at least one source message")]
    EmptySourceSelection,
    #[error("import knowledge proposal source message {message_id} appears more than once")]
    DuplicateSourceMessage { message_id: String },
    #[error("import knowledge proposal request does not match the authoritative import source")]
    ImportSourceMismatch,
    #[error("import knowledge proposal request does not match the authoritative import revision")]
    ImportRevisionMismatch,
    #[error("import knowledge proposals require a fully parsed import revision")]
    ParsedRevisionRequired,
    #[error("import source content hash changed before proposal generation")]
    SourceContentHashMismatch,
    #[error("import knowledge proposal target must be the same conversation or active FocusFrame")]
    InvalidTargetScope,
    #[error("import knowledge proposals require a model or deterministic-rule generator")]
    InvalidGenerator,
    #[error("import source snapshot {message_id} was not selected by the user")]
    UnexpectedSourceSnapshot { message_id: String },
    #[error("import source snapshot {message_id} is missing")]
    MissingSourceSnapshot { message_id: String },
    #[error("import source snapshot {message_id} appears more than once")]
    DuplicateSourceSnapshot { message_id: String },
    #[error("import source snapshot {message_id} is invalid")]
    InvalidSourceSnapshot { message_id: String },
    #[error("import knowledge proposal batch exceeds {limit} suggestions")]
    TooManySuggestions { limit: usize },
    #[error("import knowledge suggestion ordinal {ordinal} appears more than once")]
    DuplicateSuggestionOrdinal { ordinal: u32 },
    #[error("import knowledge suggestion {ordinal} has an invalid name or aliases")]
    InvalidSuggestionText { ordinal: u32 },
    #[error("import knowledge suggestion {ordinal} requires at least one EvidenceRef")]
    MissingSuggestionEvidence { ordinal: u32 },
    #[error("import knowledge suggestion {ordinal} cites unselected message {message_id}")]
    InvalidSuggestionEvidence { ordinal: u32, message_id: String },
    #[error("import knowledge suggestion {ordinal} repeats evidence message {message_id}")]
    DuplicateSuggestionEvidence { ordinal: u32, message_id: String },
    #[error("import knowledge proposal request id was reused with different input")]
    RequestIdConflict,
    #[error("import knowledge proposal decision id was reused with different input")]
    DecisionIdConflict,
    #[error("import knowledge proposal does not match the review command")]
    ProposalMismatch,
    #[error("import knowledge proposal revision conflict: expected {expected}, actual {actual}")]
    StaleProposalRevision { expected: u64, actual: u64 },
    #[error("import knowledge proposal review requires a valid user generator")]
    InvalidReviewer,
    #[error("confirm requires a kernel-authored entity id and reject must not create an entity")]
    InvalidEntityIdentity,
    #[error("import knowledge proposal review text is invalid")]
    InvalidReviewText,
    #[error("confirmed import knowledge entity is invalid: {reason}")]
    InvalidConfirmedEntity { reason: String },
}

/// Converts untrusted provider suggestions into immutable, source-grounded
/// proposals. It does not create KnowledgeEntity rows or derived indexes.
#[expect(
    clippy::too_many_arguments,
    reason = "the pure plan keeps each authoritative boundary explicit"
)]
pub fn plan_import_knowledge_proposals(
    request: &ImportKnowledgeProposalRequestInput,
    source: &ImportSource,
    revision: &ImportRevision,
    context: &ImportKnowledgeProposalTargetContext,
    source_snapshots: &[ImportKnowledgeSourceSnapshot],
    suggestions: &[ImportKnowledgeSuggestionDraft],
    generation_run_id: &str,
    generator: &GeneratorRef,
    generated_at: &str,
) -> Result<ImportKnowledgeProposalBatchProjection, ImportKnowledgeProposalError> {
    validate_request(request)?;
    validate_import_context(request, source, revision, context)?;
    require_non_empty(generation_run_id, "generationRunId")?;
    require_non_empty(generated_at, "generatedAt")?;
    if !matches!(
        generator.kind,
        GeneratorKind::Model | GeneratorKind::DeterministicRule
    ) || generator.validate().is_err()
    {
        return Err(ImportKnowledgeProposalError::InvalidGenerator);
    }
    if suggestions.len() > MAX_PROPOSALS_PER_REQUEST {
        return Err(ImportKnowledgeProposalError::TooManySuggestions {
            limit: MAX_PROPOSALS_PER_REQUEST,
        });
    }

    let snapshots = validate_source_snapshots(request, source_snapshots)?;
    let mut ordered_suggestions = suggestions.iter().collect::<Vec<_>>();
    ordered_suggestions.sort_unstable_by_key(|suggestion| suggestion.ordinal);
    let mut ordinals = HashSet::new();
    let mut proposals = Vec::with_capacity(ordered_suggestions.len());
    for suggestion in ordered_suggestions {
        if !ordinals.insert(suggestion.ordinal) {
            return Err(ImportKnowledgeProposalError::DuplicateSuggestionOrdinal {
                ordinal: suggestion.ordinal,
            });
        }
        validate_suggestion_text(suggestion)?;
        if suggestion.evidence_message_ids.is_empty() {
            return Err(ImportKnowledgeProposalError::MissingSuggestionEvidence {
                ordinal: suggestion.ordinal,
            });
        }
        if suggestion.evidence_message_ids.len() > MAX_EVIDENCE_PER_PROPOSAL {
            return Err(ImportKnowledgeProposalError::InvalidSuggestionText {
                ordinal: suggestion.ordinal,
            });
        }

        let proposal_id = format!("{}:proposal:{}", request.request_id, suggestion.ordinal);
        let mut evidence_ids = HashSet::new();
        let mut evidence_message_ids = suggestion.evidence_message_ids.clone();
        evidence_message_ids.sort_unstable();
        let mut evidence = Vec::with_capacity(evidence_message_ids.len());
        for (index, message_id) in evidence_message_ids.iter().enumerate() {
            if !evidence_ids.insert(message_id.as_str()) {
                return Err(ImportKnowledgeProposalError::DuplicateSuggestionEvidence {
                    ordinal: suggestion.ordinal,
                    message_id: message_id.clone(),
                });
            }
            let snapshot = snapshots.get(message_id.as_str()).ok_or_else(|| {
                ImportKnowledgeProposalError::InvalidSuggestionEvidence {
                    ordinal: suggestion.ordinal,
                    message_id: message_id.clone(),
                }
            })?;
            evidence.push(EvidenceRef {
                id: format!("{proposal_id}:evidence:{index}"),
                target: EvidenceTarget::ImportContent {
                    import_source_id: source.id.clone(),
                    import_revision_id: revision.id.clone(),
                    locator: snapshot.source_locator.clone(),
                },
                content_hash: Some(snapshot.content_hash.clone()),
                excerpt: Some(snapshot.excerpt.clone()),
                created_at: generated_at.into(),
            });
        }

        let mut aliases = suggestion.aliases.clone();
        aliases.sort_unstable();
        proposals.push(ImportKnowledgeEntityProposal {
            contract_version: IMPORT_KNOWLEDGE_PROPOSAL_CONTRACT_VERSION.into(),
            proposal_id,
            request_id: request.request_id.clone(),
            import_source_id: source.id.clone(),
            import_revision_id: revision.id.clone(),
            conversation_id: source.conversation_id.clone(),
            suggested_kind: suggestion.kind,
            suggested_name: suggestion.name.clone(),
            suggested_aliases: aliases,
            target_scope: request.target_scope.clone(),
            evidence,
            generator: generator.clone(),
            proposal_revision: 1,
            proposed_at: generated_at.into(),
        });
    }

    Ok(ImportKnowledgeProposalBatchProjection {
        contract_version: IMPORT_KNOWLEDGE_PROPOSAL_CONTRACT_VERSION.into(),
        request_id: request.request_id.clone(),
        import_source_id: source.id.clone(),
        import_revision_id: revision.id.clone(),
        conversation_id: source.conversation_id.clone(),
        source_content_hash: source.content_hash.clone(),
        target_scope: request.target_scope.clone(),
        generation_run_id: generation_run_id.into(),
        generator: generator.clone(),
        proposals,
        batch_revision: 1,
        requested_at: request.requested_at.clone(),
        generated_at: generated_at.into(),
    })
}

/// Builds the only allowed first KnowledgeEntity revision from an import
/// proposal. Reject decisions never create an entity.
pub fn plan_import_knowledge_proposal_review(
    input: &ImportKnowledgeProposalReviewCommandInput,
    proposal: &ImportKnowledgeEntityProposal,
    context: &ImportKnowledgeProposalTargetContext,
    entity_id: Option<&str>,
    reviewer: &GeneratorRef,
) -> Result<ImportKnowledgeProposalReviewPlan, ImportKnowledgeProposalError> {
    validate_review_command(input)?;
    validate_proposal(proposal)?;
    if input.request_id != proposal.request_id || input.proposal_id != proposal.proposal_id {
        return Err(ImportKnowledgeProposalError::ProposalMismatch);
    }
    if input.expected_proposal_revision != proposal.proposal_revision {
        return Err(ImportKnowledgeProposalError::StaleProposalRevision {
            expected: input.expected_proposal_revision,
            actual: proposal.proposal_revision,
        });
    }
    if reviewer.kind != GeneratorKind::User || reviewer.validate().is_err() {
        return Err(ImportKnowledgeProposalError::InvalidReviewer);
    }

    let (action, entity) = match &input.choice {
        ImportKnowledgeProposalReviewChoice::Confirm {
            kind,
            name,
            aliases,
        } => {
            let Some(entity_id) = entity_id.filter(|value| !value.trim().is_empty()) else {
                return Err(ImportKnowledgeProposalError::InvalidEntityIdentity);
            };
            let initial_status = review_target_status(proposal, context)?;
            validate_name_and_aliases(name, aliases)
                .map_err(|_| ImportKnowledgeProposalError::InvalidReviewText)?;
            let mut aliases = aliases.clone();
            aliases.sort_unstable();
            let evidence = proposal
                .evidence
                .iter()
                .enumerate()
                .map(|(index, evidence)| ScopedEvidenceRef {
                    id: format!("{entity_id}:evidence:{index}"),
                    evidence: evidence.clone(),
                    scope: proposal.target_scope.clone(),
                    status: initial_status,
                    revision: 1,
                    generator: reviewer.clone(),
                })
                .collect();
            let entity = KnowledgeEntity {
                contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
                id: entity_id.into(),
                kind: *kind,
                name: name.clone(),
                aliases,
                scope: proposal.target_scope.clone(),
                status: initial_status,
                revision: 1,
                evidence,
                generator: reviewer.clone(),
                created_at: input.decided_at.clone(),
                updated_at: input.decided_at.clone(),
            };
            entity.validate().map_err(|error| {
                ImportKnowledgeProposalError::InvalidConfirmedEntity {
                    reason: error.to_string(),
                }
            })?;
            (ImportKnowledgeProposalReviewAction::Confirm, Some(entity))
        }
        ImportKnowledgeProposalReviewChoice::Reject { reason } => {
            if entity_id.is_some() {
                return Err(ImportKnowledgeProposalError::InvalidEntityIdentity);
            }
            if reason.as_deref().is_some_and(|value| {
                value.trim().is_empty() || value.chars().count() > MAX_NAME_CHARS
            }) {
                return Err(ImportKnowledgeProposalError::InvalidReviewText);
            }
            (ImportKnowledgeProposalReviewAction::Reject, None)
        }
    };

    Ok(ImportKnowledgeProposalReviewPlan {
        decision: ImportKnowledgeProposalReviewProjection {
            contract_version: IMPORT_KNOWLEDGE_PROPOSAL_CONTRACT_VERSION.into(),
            decision_id: input.decision_id.clone(),
            request_id: input.request_id.clone(),
            proposal_id: input.proposal_id.clone(),
            proposal_revision: proposal.proposal_revision,
            action,
            entity_id: entity.as_ref().map(|entity| entity.id.clone()),
            entity_status: entity.as_ref().map(|entity| entity.status),
            decided_by: reviewer.clone(),
            decided_at: input.decided_at.clone(),
        },
        entity,
    })
}

/// Checks request-level idempotency before a provider run is created.
pub fn validate_import_knowledge_proposal_request_replay(
    requested: &ImportKnowledgeProposalRequestInput,
    persisted: &ImportKnowledgeProposalRequestInput,
) -> Result<(), ImportKnowledgeProposalError> {
    validate_request(requested)?;
    validate_request(persisted)?;
    let mut requested_messages = requested.selected_message_ids.clone();
    requested_messages.sort_unstable();
    let mut persisted_messages = persisted.selected_message_ids.clone();
    persisted_messages.sort_unstable();
    let matches = requested.request_id == persisted.request_id
        && requested.import_source_id == persisted.import_source_id
        && requested.import_revision_id == persisted.import_revision_id
        && requested.expected_source_content_hash == persisted.expected_source_content_hash
        && requested.target_scope == persisted.target_scope
        && requested.requested_at == persisted.requested_at
        && requested_messages == persisted_messages;
    if !matches {
        return Err(ImportKnowledgeProposalError::RequestIdConflict);
    }
    Ok(())
}

/// Checks decision-level idempotency before a proposal or entity row changes.
pub fn validate_import_knowledge_proposal_review_replay(
    requested: &ImportKnowledgeProposalReviewCommandInput,
    persisted: &ImportKnowledgeProposalReviewCommandInput,
) -> Result<(), ImportKnowledgeProposalError> {
    validate_review_command(requested)?;
    validate_review_command(persisted)?;
    let matches = requested.decision_id == persisted.decision_id
        && requested.request_id == persisted.request_id
        && requested.proposal_id == persisted.proposal_id
        && requested.expected_proposal_revision == persisted.expected_proposal_revision
        && requested.decided_at == persisted.decided_at
        && review_choices_match(&requested.choice, &persisted.choice);
    if !matches {
        return Err(ImportKnowledgeProposalError::DecisionIdConflict);
    }
    Ok(())
}

fn validate_request(
    request: &ImportKnowledgeProposalRequestInput,
) -> Result<(), ImportKnowledgeProposalError> {
    for (field, value) in [
        ("requestId", request.request_id.as_str()),
        ("importSourceId", request.import_source_id.as_str()),
        ("importRevisionId", request.import_revision_id.as_str()),
        (
            "expectedSourceContentHash",
            request.expected_source_content_hash.as_str(),
        ),
        ("requestedAt", request.requested_at.as_str()),
    ] {
        require_non_empty(value, field)?;
    }
    if request.selected_message_ids.is_empty() {
        return Err(ImportKnowledgeProposalError::EmptySourceSelection);
    }
    let mut ids = HashSet::new();
    for message_id in &request.selected_message_ids {
        if message_id.trim().is_empty() || message_id.trim() != message_id {
            return Err(ImportKnowledgeProposalError::EmptyField {
                field: "selectedMessageIds",
            });
        }
        if !ids.insert(message_id.as_str()) {
            return Err(ImportKnowledgeProposalError::DuplicateSourceMessage {
                message_id: message_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_import_context(
    request: &ImportKnowledgeProposalRequestInput,
    source: &ImportSource,
    revision: &ImportRevision,
    context: &ImportKnowledgeProposalTargetContext,
) -> Result<(), ImportKnowledgeProposalError> {
    if source.id != request.import_source_id || source.conversation_id != context.conversation_id {
        return Err(ImportKnowledgeProposalError::ImportSourceMismatch);
    }
    if source.content_hash != request.expected_source_content_hash {
        return Err(ImportKnowledgeProposalError::SourceContentHashMismatch);
    }
    if revision.id != request.import_revision_id || revision.import_source_id != source.id {
        return Err(ImportKnowledgeProposalError::ImportRevisionMismatch);
    }
    if revision.status != ImportRevisionStatus::Parsed {
        return Err(ImportKnowledgeProposalError::ParsedRevisionRequired);
    }
    let valid_scope = match &request.target_scope {
        KnowledgeScope::Conversation {
            workspace_id,
            conversation_id,
        } => workspace_id == &context.workspace_id && conversation_id == &context.conversation_id,
        KnowledgeScope::FocusFrame {
            workspace_id,
            conversation_id,
            focus_frame_id,
        } => {
            workspace_id == &context.workspace_id
                && conversation_id == &context.conversation_id
                && context.active_focus_frame_id.as_ref() == Some(focus_frame_id)
        }
        KnowledgeScope::Workspace { .. } | KnowledgeScope::Project { .. } => false,
    };
    if !valid_scope {
        return Err(ImportKnowledgeProposalError::InvalidTargetScope);
    }
    Ok(())
}

fn validate_source_snapshots<'a>(
    request: &ImportKnowledgeProposalRequestInput,
    snapshots: &'a [ImportKnowledgeSourceSnapshot],
) -> Result<HashMap<&'a str, &'a ImportKnowledgeSourceSnapshot>, ImportKnowledgeProposalError> {
    let selected = request
        .selected_message_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut indexed = HashMap::with_capacity(snapshots.len());
    for snapshot in snapshots {
        if !selected.contains(snapshot.imported_message_id.as_str()) {
            return Err(ImportKnowledgeProposalError::UnexpectedSourceSnapshot {
                message_id: snapshot.imported_message_id.clone(),
            });
        }
        if snapshot.import_revision_id != request.import_revision_id
            || snapshot.imported_message_id.trim().is_empty()
            || snapshot.source_locator.trim().is_empty()
            || snapshot.content_hash.trim().is_empty()
            || snapshot.excerpt.trim().is_empty()
        {
            return Err(ImportKnowledgeProposalError::InvalidSourceSnapshot {
                message_id: snapshot.imported_message_id.clone(),
            });
        }
        if indexed
            .insert(snapshot.imported_message_id.as_str(), snapshot)
            .is_some()
        {
            return Err(ImportKnowledgeProposalError::DuplicateSourceSnapshot {
                message_id: snapshot.imported_message_id.clone(),
            });
        }
    }
    for message_id in &request.selected_message_ids {
        if !indexed.contains_key(message_id.as_str()) {
            return Err(ImportKnowledgeProposalError::MissingSourceSnapshot {
                message_id: message_id.clone(),
            });
        }
    }
    Ok(indexed)
}

fn validate_suggestion_text(
    suggestion: &ImportKnowledgeSuggestionDraft,
) -> Result<(), ImportKnowledgeProposalError> {
    validate_name_and_aliases(&suggestion.name, &suggestion.aliases).map_err(|_| {
        ImportKnowledgeProposalError::InvalidSuggestionText {
            ordinal: suggestion.ordinal,
        }
    })
}

fn validate_name_and_aliases(name: &str, aliases: &[String]) -> Result<(), ()> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.chars().count() > MAX_NAME_CHARS
        || aliases.len() > MAX_ALIASES_PER_PROPOSAL
    {
        return Err(());
    }
    let mut values = HashSet::new();
    for alias in aliases {
        if alias.trim().is_empty()
            || alias.trim() != alias
            || alias.chars().count() > MAX_ALIAS_CHARS
            || alias == name
            || !values.insert(alias.as_str())
        {
            return Err(());
        }
    }
    Ok(())
}

fn validate_proposal(
    proposal: &ImportKnowledgeEntityProposal,
) -> Result<(), ImportKnowledgeProposalError> {
    let scope_matches = match &proposal.target_scope {
        KnowledgeScope::Conversation {
            conversation_id, ..
        }
        | KnowledgeScope::FocusFrame {
            conversation_id, ..
        } => conversation_id == &proposal.conversation_id,
        KnowledgeScope::Workspace { .. } | KnowledgeScope::Project { .. } => false,
    };
    let mut evidence_ids = HashSet::new();
    let evidence_matches = proposal.evidence.iter().all(|evidence| {
        let target_matches = matches!(
            &evidence.target,
            EvidenceTarget::ImportContent {
                import_source_id,
                import_revision_id,
                ..
            } if import_source_id == &proposal.import_source_id
                && import_revision_id == &proposal.import_revision_id
        );
        target_matches
            && evidence_ids.insert(evidence.id.as_str())
            && evidence.content_hash.is_some()
            && evidence.excerpt.is_some()
            && evidence.validate().is_ok()
    });
    if proposal.contract_version != IMPORT_KNOWLEDGE_PROPOSAL_CONTRACT_VERSION
        || proposal.proposal_revision == 0
        || proposal.proposal_id.trim().is_empty()
        || proposal.request_id.trim().is_empty()
        || proposal.import_source_id.trim().is_empty()
        || proposal.import_revision_id.trim().is_empty()
        || proposal.conversation_id.trim().is_empty()
        || proposal.proposed_at.trim().is_empty()
        || !matches!(
            proposal.generator.kind,
            GeneratorKind::Model | GeneratorKind::DeterministicRule
        )
        || proposal.generator.validate().is_err()
        || proposal.target_scope.validate().is_err()
        || !scope_matches
        || proposal.evidence.is_empty()
        || !evidence_matches
        || validate_name_and_aliases(&proposal.suggested_name, &proposal.suggested_aliases).is_err()
    {
        return Err(ImportKnowledgeProposalError::ProposalMismatch);
    }
    Ok(())
}

fn review_target_status(
    proposal: &ImportKnowledgeEntityProposal,
    context: &ImportKnowledgeProposalTargetContext,
) -> Result<KnowledgeStatus, ImportKnowledgeProposalError> {
    match &proposal.target_scope {
        KnowledgeScope::Conversation {
            workspace_id,
            conversation_id,
        } if workspace_id == &context.workspace_id
            && conversation_id == &context.conversation_id =>
        {
            Ok(KnowledgeStatus::Confirmed)
        }
        KnowledgeScope::FocusFrame {
            workspace_id,
            conversation_id,
            focus_frame_id,
        } if workspace_id == &context.workspace_id
            && conversation_id == &context.conversation_id
            && context.active_focus_frame_id.as_ref() == Some(focus_frame_id) =>
        {
            Ok(KnowledgeStatus::Candidate)
        }
        _ => Err(ImportKnowledgeProposalError::InvalidTargetScope),
    }
}

fn validate_review_command(
    input: &ImportKnowledgeProposalReviewCommandInput,
) -> Result<(), ImportKnowledgeProposalError> {
    for (field, value) in [
        ("decisionId", input.decision_id.as_str()),
        ("requestId", input.request_id.as_str()),
        ("proposalId", input.proposal_id.as_str()),
        ("decidedAt", input.decided_at.as_str()),
    ] {
        require_non_empty(value, field)?;
    }
    if input.expected_proposal_revision == 0 {
        return Err(ImportKnowledgeProposalError::StaleProposalRevision {
            expected: 0,
            actual: 1,
        });
    }
    match &input.choice {
        ImportKnowledgeProposalReviewChoice::Confirm { name, aliases, .. } => {
            validate_name_and_aliases(name, aliases)
                .map_err(|_| ImportKnowledgeProposalError::InvalidReviewText)?;
        }
        ImportKnowledgeProposalReviewChoice::Reject { reason } => {
            if reason.as_deref().is_some_and(|value| {
                value.trim().is_empty() || value.chars().count() > MAX_NAME_CHARS
            }) {
                return Err(ImportKnowledgeProposalError::InvalidReviewText);
            }
        }
    }
    Ok(())
}

fn review_choices_match(
    requested: &ImportKnowledgeProposalReviewChoice,
    persisted: &ImportKnowledgeProposalReviewChoice,
) -> bool {
    match (requested, persisted) {
        (
            ImportKnowledgeProposalReviewChoice::Confirm {
                kind: requested_kind,
                name: requested_name,
                aliases: requested_aliases,
            },
            ImportKnowledgeProposalReviewChoice::Confirm {
                kind: persisted_kind,
                name: persisted_name,
                aliases: persisted_aliases,
            },
        ) => {
            let mut requested_aliases = requested_aliases.clone();
            requested_aliases.sort_unstable();
            let mut persisted_aliases = persisted_aliases.clone();
            persisted_aliases.sort_unstable();
            requested_kind == persisted_kind
                && requested_name == persisted_name
                && requested_aliases == persisted_aliases
        }
        (
            ImportKnowledgeProposalReviewChoice::Reject {
                reason: requested_reason,
            },
            ImportKnowledgeProposalReviewChoice::Reject {
                reason: persisted_reason,
            },
        ) => requested_reason == persisted_reason,
        _ => false,
    }
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), ImportKnowledgeProposalError> {
    if value.trim().is_empty() {
        return Err(ImportKnowledgeProposalError::EmptyField { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::ImportPlatform;

    fn source() -> ImportSource {
        ImportSource {
            id: "source-1".into(),
            conversation_id: "conversation-1".into(),
            platform: ImportPlatform::Generic,
            original_file_name: Some("decisions.md".into()),
            content_hash: "sha256:source".into(),
            storage_ref: "aa/source".into(),
            created_at: "2026-09-01T00:00:00Z".into(),
        }
    }

    fn revision() -> ImportRevision {
        ImportRevision {
            id: "revision-1".into(),
            import_source_id: "source-1".into(),
            adapter_id: "generic-markdown".into(),
            adapter_version: "1".into(),
            status: ImportRevisionStatus::Parsed,
            created_at: "2026-09-01T00:00:00Z".into(),
        }
    }

    fn scope() -> KnowledgeScope {
        KnowledgeScope::Conversation {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
        }
    }

    fn context() -> ImportKnowledgeProposalTargetContext {
        ImportKnowledgeProposalTargetContext {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
            active_focus_frame_id: None,
        }
    }

    fn request(message_ids: Vec<String>) -> ImportKnowledgeProposalRequestInput {
        ImportKnowledgeProposalRequestInput {
            request_id: "proposal-request-1".into(),
            import_source_id: "source-1".into(),
            import_revision_id: "revision-1".into(),
            expected_source_content_hash: "sha256:source".into(),
            selected_message_ids: message_ids,
            target_scope: scope(),
            requested_at: "2026-09-01T01:00:00Z".into(),
        }
    }

    fn snapshot(message_id: &str, locator: &str) -> ImportKnowledgeSourceSnapshot {
        ImportKnowledgeSourceSnapshot {
            imported_message_id: message_id.into(),
            import_revision_id: "revision-1".into(),
            source_locator: locator.into(),
            content_hash: format!("sha256:{message_id}"),
            excerpt: format!("evidence from {message_id}"),
        }
    }

    fn model() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::Model,
            generator_id: "entity-extractor".into(),
            generator_version: "v1".into(),
        }
    }

    fn reviewer() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "mindscape-local-user".into(),
            generator_version: "v1".into(),
        }
    }

    fn suggestion() -> ImportKnowledgeSuggestionDraft {
        ImportKnowledgeSuggestionDraft {
            ordinal: 0,
            kind: KnowledgeEntityKind::Decision,
            name: "Keep source evidence".into(),
            aliases: vec!["Preserve provenance".into()],
            evidence_message_ids: vec!["message-1".into()],
        }
    }

    fn batch() -> ImportKnowledgeProposalBatchProjection {
        plan_import_knowledge_proposals(
            &request(vec!["message-1".into()]),
            &source(),
            &revision(),
            &context(),
            &[snapshot("message-1", "$.messages[0]")],
            &[suggestion()],
            "model-run-1",
            &model(),
            "2026-09-01T01:01:00Z",
        )
        .expect("proposal batch")
    }

    #[test]
    fn proposal_generation_resolves_model_references_to_authoritative_evidence() {
        let batch = batch();

        assert_eq!(batch.proposals.len(), 1);
        assert_eq!(
            batch.proposals[0].proposal_id,
            "proposal-request-1:proposal:0"
        );
        assert!(matches!(
            batch.proposals[0].evidence[0].target,
            EvidenceTarget::ImportContent { .. }
        ));
        assert_eq!(
            batch.proposals[0].evidence[0].content_hash.as_deref(),
            Some("sha256:message-1")
        );
    }

    #[test]
    fn proposal_generation_rejects_an_invented_evidence_message() {
        let mut draft = suggestion();
        draft.evidence_message_ids = vec!["message-invented".into()];
        let error = plan_import_knowledge_proposals(
            &request(vec!["message-1".into()]),
            &source(),
            &revision(),
            &context(),
            &[snapshot("message-1", "$.messages[0]")],
            &[draft],
            "model-run-1",
            &model(),
            "2026-09-01T01:01:00Z",
        )
        .expect_err("invented evidence");

        assert!(matches!(
            error,
            ImportKnowledgeProposalError::InvalidSuggestionEvidence { .. }
        ));
    }

    #[test]
    fn proposal_generation_rejects_a_stale_source_hash() {
        let mut stale = request(vec!["message-1".into()]);
        stale.expected_source_content_hash = "sha256:old".into();
        let error = plan_import_knowledge_proposals(
            &stale,
            &source(),
            &revision(),
            &context(),
            &[snapshot("message-1", "$.messages[0]")],
            &[suggestion()],
            "model-run-1",
            &model(),
            "2026-09-01T01:01:00Z",
        )
        .expect_err("stale import");

        assert_eq!(
            error,
            ImportKnowledgeProposalError::SourceContentHashMismatch
        );
    }

    #[test]
    fn proposal_generation_rejects_cross_conversation_scope() {
        let mut cross_scope = request(vec!["message-1".into()]);
        cross_scope.target_scope = KnowledgeScope::Conversation {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-2".into(),
        };
        let error = plan_import_knowledge_proposals(
            &cross_scope,
            &source(),
            &revision(),
            &context(),
            &[snapshot("message-1", "$.messages[0]")],
            &[suggestion()],
            "model-run-1",
            &model(),
            "2026-09-01T01:01:00Z",
        )
        .expect_err("cross conversation");

        assert_eq!(error, ImportKnowledgeProposalError::InvalidTargetScope);
    }

    #[test]
    fn proposal_request_replay_treats_selected_messages_as_a_set() {
        let requested = request(vec!["message-2".into(), "message-1".into()]);
        let persisted = request(vec!["message-1".into(), "message-2".into()]);

        validate_import_knowledge_proposal_request_replay(&requested, &persisted)
            .expect("same request");
    }

    #[test]
    fn user_confirmation_creates_the_first_confirmed_entity_revision() {
        let proposal = batch().proposals.remove(0);
        let input = ImportKnowledgeProposalReviewCommandInput {
            decision_id: "proposal-decision-1".into(),
            request_id: proposal.request_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            expected_proposal_revision: 1,
            choice: ImportKnowledgeProposalReviewChoice::Confirm {
                kind: KnowledgeEntityKind::Decision,
                name: "Keep verified source evidence".into(),
                aliases: vec!["Preserve provenance".into()],
            },
            decided_at: "2026-09-01T01:02:00Z".into(),
        };

        let plan = plan_import_knowledge_proposal_review(
            &input,
            &proposal,
            &context(),
            Some("entity-confirmed-1"),
            &reviewer(),
        )
        .expect("confirm proposal");
        let entity = plan.entity.expect("confirmed entity");

        assert_eq!(entity.status, KnowledgeStatus::Confirmed);
        assert_eq!(entity.revision, 1);
        assert_eq!(entity.generator.kind, GeneratorKind::User);
        assert_eq!(entity.evidence.len(), 1);
        assert_eq!(
            plan.decision.entity_id.as_deref(),
            Some("entity-confirmed-1")
        );
        assert_eq!(
            plan.decision.entity_status,
            Some(KnowledgeStatus::Confirmed)
        );
    }

    #[test]
    fn active_focus_confirmation_creates_a_branch_local_candidate() {
        let mut request = request(vec!["message-1".into()]);
        request.target_scope = KnowledgeScope::FocusFrame {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
            focus_frame_id: "focus-1".into(),
        };
        let focus_context = ImportKnowledgeProposalTargetContext {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
            active_focus_frame_id: Some("focus-1".into()),
        };
        let proposal = plan_import_knowledge_proposals(
            &request,
            &source(),
            &revision(),
            &focus_context,
            &[snapshot("message-1", "$.messages[0]")],
            &[suggestion()],
            "model-run-1",
            &model(),
            "2026-09-01T01:01:00Z",
        )
        .expect("focus proposal")
        .proposals
        .remove(0);
        let input = ImportKnowledgeProposalReviewCommandInput {
            decision_id: "proposal-decision-focus".into(),
            request_id: proposal.request_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            expected_proposal_revision: 1,
            choice: ImportKnowledgeProposalReviewChoice::Confirm {
                kind: proposal.suggested_kind,
                name: proposal.suggested_name.clone(),
                aliases: proposal.suggested_aliases.clone(),
            },
            decided_at: "2026-09-01T01:02:00Z".into(),
        };

        let plan = plan_import_knowledge_proposal_review(
            &input,
            &proposal,
            &focus_context,
            Some("entity-focus-candidate"),
            &reviewer(),
        )
        .expect("confirm branch proposal");

        assert_eq!(
            plan.entity.expect("candidate entity").status,
            KnowledgeStatus::Candidate
        );
        assert_eq!(
            plan.decision.entity_status,
            Some(KnowledgeStatus::Candidate)
        );
    }

    #[test]
    fn rejection_records_a_receipt_without_creating_an_entity() {
        let proposal = batch().proposals.remove(0);
        let input = ImportKnowledgeProposalReviewCommandInput {
            decision_id: "proposal-decision-reject".into(),
            request_id: proposal.request_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            expected_proposal_revision: 1,
            choice: ImportKnowledgeProposalReviewChoice::Reject {
                reason: Some("Not a durable decision".into()),
            },
            decided_at: "2026-09-01T01:02:00Z".into(),
        };

        let plan =
            plan_import_knowledge_proposal_review(&input, &proposal, &context(), None, &reviewer())
                .expect("reject proposal");

        assert_eq!(
            plan.decision.action,
            ImportKnowledgeProposalReviewAction::Reject
        );
        assert!(plan.entity.is_none());
        assert!(plan.decision.entity_id.is_none());
    }

    #[test]
    fn review_rejects_stale_proposal_revision() {
        let proposal = batch().proposals.remove(0);
        let input = ImportKnowledgeProposalReviewCommandInput {
            decision_id: "proposal-decision-stale".into(),
            request_id: proposal.request_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            expected_proposal_revision: 2,
            choice: ImportKnowledgeProposalReviewChoice::Reject { reason: None },
            decided_at: "2026-09-01T01:02:00Z".into(),
        };

        assert!(matches!(
            plan_import_knowledge_proposal_review(&input, &proposal, &context(), None, &reviewer()),
            Err(ImportKnowledgeProposalError::StaleProposalRevision { .. })
        ));
    }

    #[test]
    fn proposal_review_replay_normalizes_alias_order_and_rejects_changed_choice() {
        let mut requested = ImportKnowledgeProposalReviewCommandInput {
            decision_id: "proposal-decision-replay".into(),
            request_id: "proposal-request-1".into(),
            proposal_id: "proposal-request-1:proposal:0".into(),
            expected_proposal_revision: 1,
            choice: ImportKnowledgeProposalReviewChoice::Confirm {
                kind: KnowledgeEntityKind::Decision,
                name: "Confirmed decision".into(),
                aliases: vec!["Second".into(), "First".into()],
            },
            decided_at: "2026-09-01T01:02:00Z".into(),
        };
        let mut persisted = requested.clone();
        if let ImportKnowledgeProposalReviewChoice::Confirm { aliases, .. } = &mut persisted.choice
        {
            aliases.reverse();
        }
        validate_import_knowledge_proposal_review_replay(&requested, &persisted)
            .expect("same review");

        requested.choice = ImportKnowledgeProposalReviewChoice::Reject { reason: None };
        assert_eq!(
            validate_import_knowledge_proposal_review_replay(&requested, &persisted),
            Err(ImportKnowledgeProposalError::DecisionIdConflict)
        );
    }

    #[test]
    fn typed_contract_serializes_without_entity_identity_in_the_user_command() {
        let request =
            serde_json::to_value(request(vec!["message-1".into()])).expect("serialize request");
        let proposal = batch().proposals.remove(0);
        let review = serde_json::to_value(ImportKnowledgeProposalReviewCommandInput {
            decision_id: "decision-1".into(),
            request_id: proposal.request_id,
            proposal_id: proposal.proposal_id,
            expected_proposal_revision: 1,
            choice: ImportKnowledgeProposalReviewChoice::Confirm {
                kind: KnowledgeEntityKind::Decision,
                name: "Confirmed decision".into(),
                aliases: vec![],
            },
            decided_at: "2026-09-01T01:02:00Z".into(),
        })
        .expect("serialize review");

        assert_eq!(request["selectedMessageIds"][0], "message-1");
        assert_eq!(review["choice"]["action"], "confirm");
        assert!(review.get("entityId").is_none());
    }
}
