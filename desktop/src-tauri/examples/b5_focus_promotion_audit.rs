use std::{collections::BTreeSet, env, error::Error, fs, path::Path, process::ExitCode};

use mindscape_desktop_lib::domain::{
    FOCUS_PROMOTION_GENERATION_CONTRACT_VERSION, FocusPromotionCandidateGenerationCommandInput,
    FocusPromotionCandidateGenerationProjection, FocusPromotionDecisionAction,
    FocusPromotionDecisionCommandInput, FocusPromotionDecisionProjection,
    contracts::{
        EvidenceTarget, FocusFrame, GeneratorKind, KnowledgeEntity, KnowledgeRelation,
        KnowledgeScope, KnowledgeStatus,
    },
    validate_focus_promotion_candidate_generation_replay,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

const DATABASE_ENV: &str = "MINDSCAPE_B5_DATABASE_PATH";
const VAULT_ENV: &str = "MINDSCAPE_B5_VAULT_ROOT";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditReport {
    contract_version: &'static str,
    schema_version: i64,
    decision_table_present: bool,
    generation_table_present: bool,
    proposal_request_table_present: bool,
    proposal_review_table_present: bool,
    integrity_check: String,
    foreign_key_violations: usize,
    decision_count: usize,
    actions_present: BTreeSet<String>,
    all_four_actions_present: bool,
    knowledge_inventory: KnowledgeInventoryAudit,
    generation_count: usize,
    generations: Vec<GenerationAudit>,
    proposal_request_count: u64,
    proposal_review_count: u64,
    vault_index: VaultFileAudit,
    pending_vault_transactions: usize,
    pending_entity_delete_transactions: usize,
    pending_discussion_transactions: usize,
    pending_import_knowledge_review_transactions: usize,
    decisions: Vec<DecisionAudit>,
    violations: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationAudit {
    generation_fingerprint: String,
    focus_frame_fingerprint: String,
    candidate_count: usize,
    source_revision_count: usize,
    memory_version: u64,
    lifecycle_revision: u64,
    valid: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeInventoryAudit {
    entity_count: usize,
    confirmed_entity_count: usize,
    focus_frame_candidate_count: usize,
    embedded_evidence_count: usize,
    stored_evidence_count: u64,
    materialized_evidence_count: usize,
    evidence_vault_file_count: usize,
    import_evidence_count: usize,
    resolved_import_evidence_count: usize,
    entities: Vec<EntityProvenanceAudit>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityProvenanceAudit {
    entity_fingerprint: String,
    status: String,
    scope: &'static str,
    evidence_count: usize,
    materialized_evidence_count: usize,
    evidence_vault_file_count: usize,
    import_evidence_count: usize,
    resolved_import_evidence_count: usize,
    provenance_complete: bool,
}

struct StoredEntity {
    conversation_id: String,
    entity: KnowledgeEntity,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DecisionAudit {
    decision_fingerprint: String,
    focus_frame_fingerprint: String,
    candidate_fingerprint: String,
    action: String,
    decision_revision: u64,
    lifecycle_revision: u64,
    memory_version: u64,
    lifecycle_status: String,
    candidate_declared_in_frame: bool,
    tombstone_filters_candidate: bool,
    immutable_projection_matches_request: bool,
    source: EntityAudit,
    promoted: Option<EntityAudit>,
    promoted_evidence_matches_source: Option<bool>,
    candidate_fts_rows: u64,
    candidate_vector_rows: u64,
    candidate_relation_rows: usize,
    promoted_fts_rows: Option<u64>,
    promoted_vector_rows: Option<u64>,
    deleted_candidate_vault_links: Option<usize>,
    vector_index_status: Option<String>,
    source_vault: VaultFileAudit,
    promoted_vault: Option<VaultFileAudit>,
    valid: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityAudit {
    exists: bool,
    status: Option<String>,
    scope: Option<&'static str>,
    revision: Option<u64>,
    evidence_count: Option<usize>,
    evidence_database_rows: Option<u64>,
    evidence_vault_files: Option<usize>,
    evidence_complete: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultFileAudit {
    expected: bool,
    exists: bool,
    revision: Option<u64>,
    revision_matches: Option<bool>,
    content_fingerprint: Option<String>,
}

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let database_path = env::var_os(DATABASE_ENV)
        .ok_or("MINDSCAPE_B5_DATABASE_PATH must point to the acceptance SQLite database")?;
    let vault_root = env::var_os(VAULT_ENV)
        .ok_or("MINDSCAPE_B5_VAULT_ROOT must point to the acceptance Markdown Vault")?;
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch("PRAGMA query_only = ON; PRAGMA foreign_keys = ON;")?;
    let report = audit(&connection, Path::new(&vault_root))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if report.violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn audit(connection: &Connection, vault_root: &Path) -> Result<AuditReport, Box<dyn Error>> {
    let schema_version = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    let integrity_check = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let foreign_key_violations = {
        let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while rows.next()?.is_some() {
            count += 1;
        }
        count
    };
    let relations = load_relations(connection)?;
    let decision_table_present = table_exists(connection, "focus_promotion_decisions")?;
    let generation_table_present =
        table_exists(connection, "focus_promotion_candidate_generations")?;
    let proposal_request_table_present =
        table_exists(connection, "import_knowledge_proposal_requests")?;
    let proposal_review_table_present =
        table_exists(connection, "import_knowledge_proposal_reviews")?;
    let proposal_request_count = if proposal_request_table_present {
        connection.query_row(
            "SELECT COUNT(*) FROM import_knowledge_proposal_requests",
            [],
            |row| row.get(0),
        )?
    } else {
        0
    };
    let proposal_review_count = if proposal_review_table_present {
        connection.query_row(
            "SELECT COUNT(*) FROM import_knowledge_proposal_reviews",
            [],
            |row| row.get(0),
        )?
    } else {
        0
    };
    let generations = if generation_table_present {
        load_generation_audits(connection)?
    } else {
        Vec::new()
    };
    let stored_decisions = if decision_table_present {
        load_decisions(connection)?
    } else {
        Vec::new()
    };
    let mut actions_present = BTreeSet::new();
    let mut decisions = Vec::with_capacity(stored_decisions.len());
    let mut violations = Vec::new();
    for (request, projection) in stored_decisions {
        let decision = audit_decision(connection, vault_root, &relations, &request, &projection)?;
        actions_present.insert(decision.action.clone());
        if !decision.valid {
            violations.push(format!(
                "decision {} ({}) failed one or more persistence invariants",
                decision.decision_fingerprint, decision.action
            ));
        }
        decisions.push(decision);
    }
    let required_actions = ["confirm", "delete", "promote", "reject"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let all_four_actions_present = required_actions.is_subset(&actions_present);
    if integrity_check != "ok" {
        violations.push("SQLite integrity_check did not return ok".into());
    }
    if foreign_key_violations != 0 {
        violations.push("SQLite foreign_key_check reported violations".into());
    }
    if schema_version < 18 {
        violations.push("SQLite schema is older than the B5 import-proposal schema".into());
    }
    if !decision_table_present {
        violations.push("the B2 focus-promotion decision table is missing".into());
    }
    if !generation_table_present {
        violations.push("the B5 focus-promotion generation receipt table is missing".into());
    }
    if !proposal_request_table_present || !proposal_review_table_present {
        violations.push("the B5 import-knowledge proposal receipt tables are missing".into());
    }
    if proposal_request_count == 0 {
        violations.push("the acceptance database does not contain a real proposal request".into());
    }
    if proposal_review_count == 0 {
        violations
            .push("the acceptance database does not contain a proposal review receipt".into());
    }
    if generations.is_empty() {
        violations
            .push("the acceptance database does not contain a candidate-generation receipt".into());
    }
    if generations.iter().any(|generation| !generation.valid) {
        violations.push("one or more candidate-generation receipts are inconsistent".into());
    }
    if !all_four_actions_present {
        violations.push("the acceptance database does not contain all four B2 actions".into());
    }
    let knowledge_inventory = audit_knowledge_inventory(connection, vault_root)?;
    if knowledge_inventory.entity_count == 0 {
        violations.push("the acceptance database does not contain a real KnowledgeEntity".into());
    }
    if knowledge_inventory.import_evidence_count == 0 {
        violations
            .push("the acceptance database does not contain an import-backed EvidenceRef".into());
    }
    if knowledge_inventory.materialized_evidence_count
        != knowledge_inventory.embedded_evidence_count
    {
        violations
            .push("one or more embedded EvidenceRefs are missing from the evidence table".into());
    }
    if knowledge_inventory.evidence_vault_file_count != knowledge_inventory.embedded_evidence_count
    {
        violations.push("one or more embedded EvidenceRefs are missing Vault source pages".into());
    }
    if knowledge_inventory.resolved_import_evidence_count
        != knowledge_inventory.import_evidence_count
    {
        violations.push(
            "one or more import-backed EvidenceRefs do not resolve to the same conversation source, revision, and locator"
                .into(),
        );
    }
    let vault_index = audit_plain_vault_file(&vault_root.join("indexes/entities.md"), true)?;
    if !vault_index.exists {
        violations.push("the managed Vault entity index is missing".into());
    }
    let pending_vault_transactions =
        count_pending_transaction_directories(&vault_root.join(".transactions"))?;
    if pending_vault_transactions != 0 {
        violations.push("the Vault contains pending focus-promotion transaction journals".into());
    }
    let pending_entity_delete_transactions =
        count_pending_transaction_directories(&vault_root.join(".entity-delete-transactions"))?;
    if pending_entity_delete_transactions != 0 {
        violations.push("the Vault contains pending knowledge-delete transaction journals".into());
    }
    let pending_discussion_transactions =
        count_pending_transaction_directories(&vault_root.join(".discussion-transactions"))?;
    if pending_discussion_transactions != 0 {
        violations.push("the Vault contains pending DiscussionLog transaction journals".into());
    }
    let pending_import_knowledge_review_transactions = count_pending_transaction_directories(
        &vault_root.join(".import-knowledge-review-transactions"),
    )?;
    if pending_import_knowledge_review_transactions != 0 {
        violations.push("the Vault contains pending import-knowledge review journals".into());
    }

    Ok(AuditReport {
        contract_version: "mindscape.b5-focus-promotion-audit.v1",
        schema_version,
        decision_table_present,
        generation_table_present,
        proposal_request_table_present,
        proposal_review_table_present,
        integrity_check,
        foreign_key_violations,
        decision_count: decisions.len(),
        actions_present,
        all_four_actions_present,
        knowledge_inventory,
        generation_count: generations.len(),
        generations,
        proposal_request_count,
        proposal_review_count,
        vault_index,
        pending_vault_transactions,
        pending_entity_delete_transactions,
        pending_discussion_transactions,
        pending_import_knowledge_review_transactions,
        decisions,
        violations,
    })
}

fn load_generation_audits(connection: &Connection) -> Result<Vec<GenerationAudit>, Box<dyn Error>> {
    let mut statement = connection.prepare(
        "SELECT request_json, projection_json
         FROM focus_promotion_candidate_generations ORDER BY generation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.map(|row| {
        let (request_json, projection_json) = row?;
        let input: FocusPromotionCandidateGenerationCommandInput =
            serde_json::from_str(&request_json)?;
        let projection: FocusPromotionCandidateGenerationProjection =
            serde_json::from_str(&projection_json)?;
        let mut expected_refs = input.candidate_refs.clone();
        expected_refs.sort_unstable();
        let source_revisions_match = projection.source_entity_revisions.len()
            == expected_refs.len()
            && projection
                .source_entity_revisions
                .iter()
                .zip(&expected_refs)
                .all(|(source, expected)| {
                    source.candidate_ref == *expected && source.entity_revision > 0
                });
        let expected_memory_version = input.expected_memory_version.checked_add(1);
        let expected_lifecycle_revision = input.expected_lifecycle_revision.checked_add(1);
        let valid = validate_focus_promotion_candidate_generation_replay(&input, &input).is_ok()
            && projection.contract_version == FOCUS_PROMOTION_GENERATION_CONTRACT_VERSION
            && projection.generation_id == input.generation_id
            && projection.focus_frame_id == input.focus_frame_id
            && !projection.conversation_id.trim().is_empty()
            && projection.candidate_refs == expected_refs
            && source_revisions_match
            && Some(projection.memory_version) == expected_memory_version
            && Some(projection.lifecycle_revision) == expected_lifecycle_revision
            && projection.selected_by.kind == GeneratorKind::User
            && projection.selected_by.validate().is_ok()
            && projection.generated_at == input.generated_at;
        Ok(GenerationAudit {
            generation_fingerprint: fingerprint(&projection.generation_id),
            focus_frame_fingerprint: fingerprint(&projection.focus_frame_id),
            candidate_count: projection.candidate_refs.len(),
            source_revision_count: projection.source_entity_revisions.len(),
            memory_version: projection.memory_version,
            lifecycle_revision: projection.lifecycle_revision,
            valid,
        })
    })
    .collect()
}

fn audit_knowledge_inventory(
    connection: &Connection,
    vault_root: &Path,
) -> Result<KnowledgeInventoryAudit, Box<dyn Error>> {
    let stored_entities = load_all_entities(connection)?;
    let stored_evidence_count =
        connection.query_row("SELECT COUNT(*) FROM evidence_refs", [], |row| row.get(0))?;
    let mut confirmed_entity_count = 0;
    let mut focus_frame_candidate_count = 0;
    let mut embedded_evidence_count = 0;
    let mut materialized_evidence_count = 0;
    let mut evidence_vault_file_count = 0;
    let mut import_evidence_count = 0;
    let mut resolved_import_evidence_count = 0;
    let mut entities = Vec::with_capacity(stored_entities.len());

    for stored in &stored_entities {
        let entity = &stored.entity;
        if entity.status == KnowledgeStatus::Confirmed {
            confirmed_entity_count += 1;
        }
        if matches!(entity.scope, KnowledgeScope::FocusFrame { .. })
            && matches!(
                entity.status,
                KnowledgeStatus::Candidate | KnowledgeStatus::Inferred
            )
        {
            focus_frame_candidate_count += 1;
        }

        let mut entity_materialized_evidence_count = 0;
        let mut entity_evidence_vault_file_count = 0;
        let mut entity_import_evidence_count = 0;
        let mut entity_resolved_import_evidence_count = 0;
        for scoped in &entity.evidence {
            if evidence_row_exists(connection, &stored.conversation_id, &scoped.evidence.id)? {
                entity_materialized_evidence_count += 1;
            }
            if is_safe_stable_id(&scoped.evidence.id)
                && vault_root
                    .join("sources")
                    .join(format!("{}.md", scoped.evidence.id))
                    .is_file()
            {
                entity_evidence_vault_file_count += 1;
            }
            if let EvidenceTarget::ImportContent {
                import_source_id,
                import_revision_id,
                locator,
            } = &scoped.evidence.target
            {
                entity_import_evidence_count += 1;
                if import_evidence_resolves(
                    connection,
                    &stored.conversation_id,
                    import_source_id,
                    import_revision_id,
                    locator,
                )? {
                    entity_resolved_import_evidence_count += 1;
                }
            }
        }

        let evidence_count = entity.evidence.len();
        embedded_evidence_count += evidence_count;
        materialized_evidence_count += entity_materialized_evidence_count;
        evidence_vault_file_count += entity_evidence_vault_file_count;
        import_evidence_count += entity_import_evidence_count;
        resolved_import_evidence_count += entity_resolved_import_evidence_count;
        entities.push(EntityProvenanceAudit {
            entity_fingerprint: fingerprint(&entity.id),
            status: json_atom(&entity.status)?,
            scope: scope_name(&entity.scope),
            evidence_count,
            materialized_evidence_count: entity_materialized_evidence_count,
            evidence_vault_file_count: entity_evidence_vault_file_count,
            import_evidence_count: entity_import_evidence_count,
            resolved_import_evidence_count: entity_resolved_import_evidence_count,
            provenance_complete: evidence_count > 0
                && entity_materialized_evidence_count == evidence_count
                && entity_evidence_vault_file_count == evidence_count
                && entity_import_evidence_count > 0
                && entity_resolved_import_evidence_count == entity_import_evidence_count,
        });
    }

    Ok(KnowledgeInventoryAudit {
        entity_count: stored_entities.len(),
        confirmed_entity_count,
        focus_frame_candidate_count,
        embedded_evidence_count,
        stored_evidence_count,
        materialized_evidence_count,
        evidence_vault_file_count,
        import_evidence_count,
        resolved_import_evidence_count,
        entities,
    })
}

fn load_all_entities(connection: &Connection) -> Result<Vec<StoredEntity>, Box<dyn Error>> {
    let mut statement = connection
        .prepare("SELECT conversation_id, entity_json FROM knowledge_entities ORDER BY id")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.map(|row| {
        let (conversation_id, entity_json) = row?;
        Ok(StoredEntity {
            conversation_id,
            entity: serde_json::from_str(&entity_json)?,
        })
    })
    .collect()
}

fn evidence_row_exists(
    connection: &Connection,
    conversation_id: &str,
    evidence_id: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM evidence_refs WHERE id = ?1 AND conversation_id = ?2)",
        [evidence_id, conversation_id],
        |row| row.get(0),
    )
}

fn import_evidence_resolves(
    connection: &Connection,
    conversation_id: &str,
    import_source_id: &str,
    import_revision_id: &str,
    locator: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
             SELECT 1
             FROM import_sources source
             JOIN import_revisions revision ON revision.import_source_id = source.id
             JOIN imported_messages message ON message.import_revision_id = revision.id
             WHERE source.id = ?1
               AND source.conversation_id = ?2
               AND revision.id = ?3
               AND message.source_locator = ?4
         )",
        [
            import_source_id,
            conversation_id,
            import_revision_id,
            locator,
        ],
        |row| row.get(0),
    )
}

fn table_exists(connection: &Connection, table_name: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table_name],
        |row| row.get(0),
    )
}

fn load_decisions(
    connection: &Connection,
) -> Result<
    Vec<(
        FocusPromotionDecisionCommandInput,
        FocusPromotionDecisionProjection,
    )>,
    Box<dyn Error>,
> {
    let mut statement = connection.prepare(
        "SELECT request_json, projection_json FROM focus_promotion_decisions ORDER BY decision_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.map(|row| {
        let (request, projection) = row?;
        Ok((
            serde_json::from_str(&request)?,
            serde_json::from_str(&projection)?,
        ))
    })
    .collect()
}

fn load_relations(connection: &Connection) -> Result<Vec<KnowledgeRelation>, Box<dyn Error>> {
    let mut statement = connection.prepare("SELECT relation_json FROM knowledge_relations")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
}

fn audit_decision(
    connection: &Connection,
    vault_root: &Path,
    relations: &[KnowledgeRelation],
    request: &FocusPromotionDecisionCommandInput,
    projection: &FocusPromotionDecisionProjection,
) -> Result<DecisionAudit, Box<dyn Error>> {
    let action = json_atom(&projection.action)?;
    let source = load_entity(connection, &projection.candidate_ref)?;
    let promoted = match projection.promoted_entity_id.as_deref() {
        Some(id) => load_entity(connection, id)?,
        None => None,
    };
    let (lifecycle_status, candidate_declared_in_frame) = connection
        .query_row(
            "SELECT status, frame_json FROM focus_frame_lifecycle WHERE focus_frame_id = ?1",
            [&projection.focus_frame_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map(|(status, frame)| -> Result<_, Box<dyn Error>> {
            let status: serde_json::Value = serde_json::from_str(&status)?;
            let frame: FocusFrame = serde_json::from_str(&frame)?;
            Ok((
                status.as_str().unwrap_or("invalid").to_owned(),
                frame
                    .memory_scope
                    .promote_refs
                    .contains(&projection.candidate_ref),
            ))
        })??;
    let candidate_fts_rows = count_rows(
        connection,
        "SELECT COUNT(*) FROM knowledge_entities_fts WHERE entity_id = ?1",
        &projection.candidate_ref,
    )?;
    let candidate_vector_rows = count_rows(
        connection,
        "SELECT COUNT(*) FROM knowledge_vector_records WHERE entity_id = ?1",
        &projection.candidate_ref,
    )?;
    let candidate_relation_rows = relations
        .iter()
        .filter(|relation| {
            relation.source_entity_id == projection.candidate_ref
                || relation.target_entity_id == projection.candidate_ref
        })
        .count();
    let promoted_fts_rows = projection
        .promoted_entity_id
        .as_deref()
        .map(|id| {
            count_rows(
                connection,
                "SELECT COUNT(*) FROM knowledge_entities_fts WHERE entity_id = ?1",
                id,
            )
        })
        .transpose()?;
    let promoted_vector_rows = projection
        .promoted_entity_id
        .as_deref()
        .map(|id| {
            count_rows(
                connection,
                "SELECT COUNT(*) FROM knowledge_vector_records WHERE entity_id = ?1",
                id,
            )
        })
        .transpose()?;
    let deleted_candidate_vault_links = (projection.action == FocusPromotionDecisionAction::Delete)
        .then(|| count_entity_vault_links(vault_root, &projection.candidate_ref))
        .transpose()?;
    let vector_index_status = connection
        .query_row(
            "SELECT status FROM knowledge_vector_index_states WHERE conversation_id = ?1",
            [&projection.conversation_id],
            |row| row.get(0),
        )
        .optional()?;
    let immutable_projection_matches_request = projection.decision_id == request.decision_id
        && projection.focus_frame_id == request.focus_frame_id
        && projection.candidate_ref == request.candidate_ref
        && projection.action == request.action
        && projection.target_scope == request.target_scope
        && projection.promoted_entity_id == request.promoted_entity_id
        && projection.memory_version == request.expected_memory_version
        && projection.lifecycle_revision == request.expected_lifecycle_revision
        && projection.decision_revision == 1
        && projection.decided_at == request.decided_at;
    let source_expected = projection.action != FocusPromotionDecisionAction::Delete;
    let source_revision = source.as_ref().map(|entity| entity.revision);
    let source_vault = audit_entity_vault_file(
        vault_root,
        &projection.candidate_ref,
        source_expected,
        source_revision,
    )?;
    let promoted_vault = projection
        .promoted_entity_id
        .as_deref()
        .map(|id| {
            audit_entity_vault_file(
                vault_root,
                id,
                true,
                promoted.as_ref().map(|entity| entity.revision),
            )
        })
        .transpose()?;
    let promoted_evidence_matches_source = promoted.as_ref().map(|promoted| {
        source
            .as_ref()
            .is_some_and(|source| promoted.evidence == source.evidence)
    });
    let source_audit = entity_audit(connection, vault_root, source.as_ref())?;
    let promoted_audit = promoted
        .as_ref()
        .map(|entity| entity_audit(connection, vault_root, Some(entity)))
        .transpose()?;
    let entity_state_valid = match projection.action {
        FocusPromotionDecisionAction::Confirm => {
            source
                .as_ref()
                .is_some_and(|entity| entity.status == KnowledgeStatus::Confirmed)
                && source_audit.evidence_complete == Some(true)
        }
        FocusPromotionDecisionAction::Promote => {
            source
                .as_ref()
                .is_some_and(|entity| entity.status == KnowledgeStatus::Confirmed)
                && promoted
                    .as_ref()
                    .is_some_and(|entity| entity.status == KnowledgeStatus::Confirmed)
                && promoted_evidence_matches_source == Some(true)
                && source_audit.evidence_complete == Some(true)
                && promoted_audit
                    .as_ref()
                    .is_some_and(|entity| entity.evidence_complete == Some(true))
        }
        FocusPromotionDecisionAction::Reject => {
            source
                .as_ref()
                .is_some_and(|entity| entity.status == KnowledgeStatus::Rejected)
                && source_audit.evidence_complete == Some(true)
        }
        FocusPromotionDecisionAction::Delete => source.is_none(),
    };
    let retrieval_state_valid = match projection.action {
        FocusPromotionDecisionAction::Confirm | FocusPromotionDecisionAction::Promote => {
            candidate_fts_rows == 1
                && (candidate_vector_rows == 1 || vector_index_status.as_deref() == Some("stale"))
                && promoted_fts_rows.is_none_or(|rows| rows == 1)
                && promoted_vector_rows
                    .is_none_or(|rows| rows == 1 || vector_index_status.as_deref() == Some("stale"))
        }
        FocusPromotionDecisionAction::Reject | FocusPromotionDecisionAction::Delete => {
            candidate_fts_rows == 0 && candidate_vector_rows == 0
        }
    };
    let relation_state_valid = projection.action != FocusPromotionDecisionAction::Delete
        || (candidate_relation_rows == 0 && deleted_candidate_vault_links == Some(0));
    let vault_state_valid = source_vault.exists == source_expected
        && source_vault.revision_matches.unwrap_or(true)
        && promoted_vault
            .as_ref()
            .is_none_or(|audit| audit.exists && audit.revision_matches == Some(true));
    let valid = immutable_projection_matches_request
        && candidate_declared_in_frame
        && entity_state_valid
        && retrieval_state_valid
        && relation_state_valid
        && vault_state_valid;

    Ok(DecisionAudit {
        decision_fingerprint: fingerprint(&projection.decision_id),
        focus_frame_fingerprint: fingerprint(&projection.focus_frame_id),
        candidate_fingerprint: fingerprint(&projection.candidate_ref),
        action,
        decision_revision: projection.decision_revision,
        lifecycle_revision: projection.lifecycle_revision,
        memory_version: projection.memory_version,
        lifecycle_status,
        candidate_declared_in_frame,
        tombstone_filters_candidate: true,
        immutable_projection_matches_request,
        source: source_audit,
        promoted: promoted_audit,
        promoted_evidence_matches_source,
        candidate_fts_rows,
        candidate_vector_rows,
        candidate_relation_rows,
        promoted_fts_rows,
        promoted_vector_rows,
        deleted_candidate_vault_links,
        vector_index_status,
        source_vault,
        promoted_vault,
        valid,
    })
}

fn load_entity(
    connection: &Connection,
    entity_id: &str,
) -> Result<Option<KnowledgeEntity>, Box<dyn Error>> {
    let value = connection
        .query_row(
            "SELECT entity_json FROM knowledge_entities WHERE id = ?1",
            [entity_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    value
        .map(|json| serde_json::from_str(&json).map_err(Into::into))
        .transpose()
}

fn entity_audit(
    connection: &Connection,
    vault_root: &Path,
    entity: Option<&KnowledgeEntity>,
) -> Result<EntityAudit, Box<dyn Error>> {
    Ok(match entity {
        Some(entity) => {
            let evidence_database_rows =
                entity.evidence.iter().try_fold(0_u64, |total, item| {
                    count_rows(
                        connection,
                        "SELECT COUNT(*) FROM evidence_refs WHERE id = ?1",
                        &item.evidence.id,
                    )
                    .map(|count| total + count)
                })?;
            let evidence_vault_files = entity
                .evidence
                .iter()
                .filter(|item| {
                    is_safe_stable_id(&item.evidence.id)
                        && vault_root
                            .join("sources")
                            .join(format!("{}.md", item.evidence.id))
                            .is_file()
                })
                .count();
            let expected = entity.evidence.len();
            EntityAudit {
                exists: true,
                status: Some(json_atom(&entity.status)?),
                scope: Some(scope_name(&entity.scope)),
                revision: Some(entity.revision),
                evidence_count: Some(expected),
                evidence_database_rows: Some(evidence_database_rows),
                evidence_vault_files: Some(evidence_vault_files),
                evidence_complete: Some(
                    evidence_database_rows == u64::try_from(expected)?
                        && evidence_vault_files == expected,
                ),
            }
        }
        None => EntityAudit {
            exists: false,
            status: None,
            scope: None,
            revision: None,
            evidence_count: None,
            evidence_database_rows: None,
            evidence_vault_files: None,
            evidence_complete: None,
        },
    })
}

fn count_pending_transaction_directories(path: &Path) -> Result<usize, Box<dyn Error>> {
    if !path.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        if entry?.file_type()?.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

fn count_entity_vault_links(vault_root: &Path, entity_id: &str) -> Result<usize, Box<dyn Error>> {
    let link_prefix = format!("[[{entity_id}|");
    let mut count = 0;
    for entry in fs::read_dir(vault_root.join("entities"))? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("md")
            && fs::read_to_string(entry.path())?.contains(&link_prefix)
        {
            count += 1;
        }
    }
    Ok(count)
}

fn audit_entity_vault_file(
    vault_root: &Path,
    entity_id: &str,
    expected: bool,
    expected_revision: Option<u64>,
) -> Result<VaultFileAudit, Box<dyn Error>> {
    if !is_safe_stable_id(entity_id) {
        return Ok(VaultFileAudit {
            expected,
            exists: false,
            revision: None,
            revision_matches: Some(false),
            content_fingerprint: None,
        });
    }
    let path = vault_root.join("entities").join(format!("{entity_id}.md"));
    let mut audit = audit_plain_vault_file(&path, expected)?;
    if audit.exists {
        let content = fs::read_to_string(path)?;
        audit.revision = content.lines().find_map(|line| {
            line.strip_prefix("entityRevision: ")
                .and_then(|value| value.parse().ok())
        });
        audit.revision_matches = expected_revision.map(|expected| audit.revision == Some(expected));
    }
    Ok(audit)
}

fn audit_plain_vault_file(path: &Path, expected: bool) -> Result<VaultFileAudit, Box<dyn Error>> {
    if !path.is_file() {
        return Ok(VaultFileAudit {
            expected,
            exists: false,
            revision: None,
            revision_matches: None,
            content_fingerprint: None,
        });
    }
    let content = fs::read(path)?;
    Ok(VaultFileAudit {
        expected,
        exists: true,
        revision: None,
        revision_matches: None,
        content_fingerprint: Some(fingerprint_bytes(&content)),
    })
}

fn count_rows(connection: &Connection, sql: &str, id: &str) -> rusqlite::Result<u64> {
    connection.query_row(sql, [id], |row| row.get(0))
}

fn json_atom(value: &impl Serialize) -> Result<String, serde_json::Error> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .unwrap_or("invalid")
        .to_owned())
}

fn scope_name(scope: &KnowledgeScope) -> &'static str {
    match scope {
        KnowledgeScope::Workspace { .. } => "workspace",
        KnowledgeScope::Project { .. } => "project",
        KnowledgeScope::Conversation { .. } => "conversation",
        KnowledgeScope::FocusFrame { .. } => "focusFrame",
    }
}

fn is_safe_stable_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn fingerprint(value: &str) -> String {
    fingerprint_bytes(value.as_bytes())
}

fn fingerprint_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
        .chars()
        .take(12)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_provenance_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory audit database");
        connection
            .execute_batch(
                "CREATE TABLE import_sources (
                    id TEXT PRIMARY KEY,
                    conversation_id TEXT NOT NULL
                 );
                 CREATE TABLE import_revisions (
                    id TEXT PRIMARY KEY,
                    import_source_id TEXT NOT NULL
                 );
                 CREATE TABLE imported_messages (
                    id TEXT PRIMARY KEY,
                    import_revision_id TEXT NOT NULL,
                    source_locator TEXT NOT NULL
                 );
                 INSERT INTO import_sources (id, conversation_id)
                    VALUES ('source-1', 'conversation-1');
                 INSERT INTO import_revisions (id, import_source_id)
                    VALUES ('revision-1', 'source-1');
                 INSERT INTO imported_messages (id, import_revision_id, source_locator)
                    VALUES ('message-1', 'revision-1', '$.messages[0]');",
            )
            .expect("provenance fixture");
        connection
    }

    #[test]
    fn fingerprints_are_stable_and_do_not_reveal_the_original_id() {
        let value = fingerprint("decision-sensitive-1");

        assert_eq!(value, "7e8cb5ac6be4");
        assert!(!value.contains("decision-sensitive-1"));
    }

    #[test]
    fn stable_id_validation_rejects_vault_path_escape() {
        assert!(!is_safe_stable_id("../entity"));
    }

    #[test]
    fn import_evidence_resolves_only_for_the_stored_source_revision_and_locator() {
        let connection = import_provenance_connection();

        let resolved = import_evidence_resolves(
            &connection,
            "conversation-1",
            "source-1",
            "revision-1",
            "$.messages[0]",
        )
        .expect("resolve import evidence");

        assert!(resolved);
    }

    #[test]
    fn import_evidence_rejects_a_source_from_another_conversation() {
        let connection = import_provenance_connection();

        let resolved = import_evidence_resolves(
            &connection,
            "conversation-2",
            "source-1",
            "revision-1",
            "$.messages[0]",
        )
        .expect("resolve import evidence");

        assert!(!resolved);
    }
}
