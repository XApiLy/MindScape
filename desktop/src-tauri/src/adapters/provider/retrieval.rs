use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::domain::{
    KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION, KnowledgeEmbeddingProvenance,
    KnowledgeRetrievalAvailability, KnowledgeRetrievalCandidateProjection,
    KnowledgeRetrievalNotice, KnowledgeRetrievalProjection, KnowledgeRetrievalSource,
    OmittedKnowledgeRef,
    contracts::{EvidenceRef, KnowledgeEntity, KnowledgeRelation, KnowledgeStatus},
};

use super::embedding::{
    EmbeddingAdapter, EmbeddingRecord, LocalHashEmbedding, LocalVectorIndex, RetrievalAvailability,
    RetrievalCandidate, RetrievalResult, RetrievalSource,
};

pub const HYBRID_RETRIEVAL_VERSION: &str = "mindscape-hybrid-retrieval-v1";
pub const KNOWLEDGE_ENTITY_INDEX_VERSION: &str = "mindscape-knowledge-entity-index-v1";

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeFullTextMatch {
    pub id: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct KnowledgeVectorSnapshot {
    pub availability: RetrievalAvailability,
    pub records: Vec<EmbeddingRecord>,
}

#[derive(Debug, Clone)]
pub struct SemanticQueryEmbedding {
    pub model_version: String,
    pub dimensions: usize,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct HydratedRetrievalCandidate {
    pub entity: KnowledgeEntity,
    pub evidence: Vec<EvidenceRef>,
    pub estimated_tokens: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RetrievalProjectionError {
    #[error("knowledge retrieval query and limit must be valid")]
    InvalidQuery,
    #[error("knowledge retrieval fact is invalid: {0}")]
    InvalidFact(String),
    #[error("retrieval candidate {id} has a non-finite or out-of-range score")]
    InvalidScore { id: String },
    #[error("retrieval candidate {id} was hydrated with entity {entity_id}")]
    EntityMismatch { id: String, entity_id: String },
    #[error("retrieval projection is invalid: {0}")]
    InvalidProjection(String),
}

pub fn knowledge_search_text(entity: &KnowledgeEntity) -> String {
    let raw = std::iter::once(entity.name.as_str())
        .chain(entity.aliases.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    normalize_retrieval_text(&raw)
}

pub fn normalize_retrieval_text(value: &str) -> String {
    let mut tokens = Vec::new();
    let mut ascii_word = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            ascii_word.push(character.to_ascii_lowercase());
            continue;
        }
        if !ascii_word.is_empty() {
            tokens.push(std::mem::take(&mut ascii_word));
        }
        if character.is_alphanumeric() {
            tokens.push(character.to_lowercase().collect());
        }
    }
    if !ascii_word.is_empty() {
        tokens.push(ascii_word);
    }
    tokens.join(" ")
}

pub fn build_knowledge_embedding_record(
    entity: &KnowledgeEntity,
) -> Result<EmbeddingRecord, RetrievalProjectionError> {
    let adapter = LocalHashEmbedding;
    let mut index = LocalVectorIndex::default();
    index.upsert(
        &adapter,
        entity.id.clone(),
        &knowledge_search_text(entity),
        knowledge_entity_source_hash(entity)?,
        KNOWLEDGE_ENTITY_INDEX_VERSION,
    );
    index
        .snapshot()
        .into_iter()
        .next()
        .ok_or_else(|| RetrievalProjectionError::InvalidFact("embedding record is absent".into()))
}

pub fn build_knowledge_embedding_record_from_vector(
    entity: &KnowledgeEntity,
    model_version: &str,
    dimensions: usize,
    vector: Vec<f32>,
) -> Result<EmbeddingRecord, RetrievalProjectionError> {
    let record = EmbeddingRecord {
        id: entity.id.clone(),
        metadata: super::embedding::EmbeddingMetadata {
            model_version: model_version.into(),
            dimensions,
            source_hash: knowledge_entity_source_hash(entity)?,
            chunk_version: KNOWLEDGE_ENTITY_INDEX_VERSION.into(),
        },
        vector,
    };
    let mut index = LocalVectorIndex::default();
    index
        .upsert_precomputed(record.clone())
        .map_err(|error| RetrievalProjectionError::InvalidFact(error.to_string()))?;
    Ok(record)
}

pub fn retrieve_validated_knowledge(
    conversation_id: &str,
    query: &str,
    limit: usize,
    entities: &[KnowledgeEntity],
    relations: &[KnowledgeRelation],
    full_text_matches: Vec<KnowledgeFullTextMatch>,
    vector_snapshot: KnowledgeVectorSnapshot,
) -> Result<KnowledgeRetrievalProjection, RetrievalProjectionError> {
    retrieve_validated_knowledge_with_semantic(
        conversation_id,
        query,
        limit,
        entities,
        relations,
        full_text_matches,
        vector_snapshot,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn retrieve_validated_knowledge_with_semantic(
    conversation_id: &str,
    query: &str,
    limit: usize,
    entities: &[KnowledgeEntity],
    relations: &[KnowledgeRelation],
    full_text_matches: Vec<KnowledgeFullTextMatch>,
    vector_snapshot: KnowledgeVectorSnapshot,
    semantic_query: Option<&SemanticQueryEmbedding>,
) -> Result<KnowledgeRetrievalProjection, RetrievalProjectionError> {
    let normalized_query = normalize_retrieval_text(query);
    if conversation_id.trim().is_empty() || normalized_query.is_empty() || limit == 0 || limit > 50
    {
        return Err(RetrievalProjectionError::InvalidQuery);
    }

    let mut hydrated = HashMap::new();
    let mut expected_source_hashes = HashMap::new();
    for entity in entities {
        entity
            .validate_for_conversation(conversation_id)
            .map_err(|error| RetrievalProjectionError::InvalidFact(error.to_string()))?;
        if entity.status != KnowledgeStatus::Confirmed {
            continue;
        }
        let search_text = knowledge_search_text(entity);
        let source_hash = knowledge_entity_source_hash(entity)?;
        expected_source_hashes.insert(entity.id.clone(), source_hash);
        hydrated.insert(
            entity.id.clone(),
            HydratedRetrievalCandidate {
                entity: entity.clone(),
                evidence: entity
                    .evidence
                    .iter()
                    .filter(|reference| reference.status == KnowledgeStatus::Confirmed)
                    .map(|reference| reference.evidence.clone())
                    .collect(),
                estimated_tokens: estimate_tokens(&search_text)?,
            },
        );
    }
    for relation in relations {
        relation
            .validate_for_conversation(conversation_id)
            .map_err(|error| RetrievalProjectionError::InvalidFact(error.to_string()))?;
    }

    let expanded_limit = limit.saturating_mul(3).min(150);
    let adapter = LocalHashEmbedding;
    let snapshot_is_complete = vector_snapshot.records.len() == expected_source_hashes.len()
        && vector_snapshot.records.iter().all(|record| {
            expected_source_hashes
                .get(&record.id)
                .is_some_and(|expected_hash| {
                    record.metadata.source_hash == *expected_hash
                        && record.metadata.chunk_version == KNOWLEDGE_ENTITY_INDEX_VERSION
                })
        });
    let (model_version, dimensions) = semantic_query
        .map(|embedding| (embedding.model_version.as_str(), embedding.dimensions))
        .unwrap_or_else(|| (adapter.model_version(), adapter.dimensions()));
    let restored_index = (vector_snapshot.availability == RetrievalAvailability::Available
        && snapshot_is_complete)
        .then(|| {
            LocalVectorIndex::restore_contract(model_version, dimensions, vector_snapshot.records)
        })
        .transpose()
        .ok()
        .flatten();
    let vector_status = if restored_index.is_some() {
        RetrievalAvailability::Available
    } else {
        RetrievalAvailability::Unavailable
    };
    let vector_candidates = restored_index
        .as_ref()
        .map(|index| {
            let matches = semantic_query.map_or_else(
                || index.search(&adapter, &normalized_query, expanded_limit),
                |embedding| index.search_precomputed(&embedding.vector, expanded_limit),
            );
            matches
                .iter()
                .filter(|candidate| candidate.score > 0.0)
                .map(RetrievalCandidate::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let full_text_candidates = full_text_matches
        .into_iter()
        .map(|hit| RetrievalCandidate {
            id: hit.id,
            score: hit.score,
            sources: vec![RetrievalSource::FullText],
            embedding: None,
        })
        .collect::<Vec<_>>();
    let eligible_ids = hydrated.keys().map(String::as_str).collect::<HashSet<_>>();
    let seed_ids = vector_candidates
        .iter()
        .chain(&full_text_candidates)
        .filter(|candidate| eligible_ids.contains(candidate.id.as_str()))
        .map(|candidate| candidate.id.as_str())
        .collect::<HashSet<_>>();
    let relation_candidates = relations
        .iter()
        .filter(|relation| relation.status == KnowledgeStatus::Confirmed)
        .filter_map(|relation| {
            let neighbor = if seed_ids.contains(relation.source_entity_id.as_str()) {
                relation.target_entity_id.as_str()
            } else if seed_ids.contains(relation.target_entity_id.as_str()) {
                relation.source_entity_id.as_str()
            } else {
                return None;
            };
            eligible_ids.contains(neighbor).then(|| RetrievalCandidate {
                id: neighbor.into(),
                score: 0.55,
                sources: vec![RetrievalSource::Relation],
                embedding: None,
            })
        })
        .collect::<Vec<_>>();

    let merged = super::embedding::merge_retrieval_candidates(
        vector_status,
        vector_candidates,
        full_text_candidates,
        relation_candidates,
        limit,
    );
    project_retrieval_result(merged, &hydrated)
}

pub(crate) fn knowledge_entity_source_hash(
    entity: &KnowledgeEntity,
) -> Result<String, RetrievalProjectionError> {
    let serialized = serde_json::to_vec(entity)
        .map_err(|error| RetrievalProjectionError::InvalidFact(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(serialized)))
}

fn estimate_tokens(search_text: &str) -> Result<i64, RetrievalProjectionError> {
    let estimate = search_text.chars().count().saturating_add(3) / 4;
    i64::try_from(estimate.max(1))
        .map_err(|error| RetrievalProjectionError::InvalidFact(error.to_string()))
}

pub fn project_retrieval_result(
    result: RetrievalResult,
    hydrated: &HashMap<String, HydratedRetrievalCandidate>,
) -> Result<KnowledgeRetrievalProjection, RetrievalProjectionError> {
    let mut candidates = Vec::new();
    let mut omitted = Vec::new();
    for candidate in result.candidates {
        let Some(fact) = hydrated.get(&candidate.id) else {
            omitted.push(OmittedKnowledgeRef {
                reference_id: candidate.id,
                reason: "candidate is absent from the validated knowledge fact source".into(),
            });
            continue;
        };
        if fact.entity.id != candidate.id {
            return Err(RetrievalProjectionError::EntityMismatch {
                id: candidate.id,
                entity_id: fact.entity.id.clone(),
            });
        }
        candidates.push(KnowledgeRetrievalCandidateProjection {
            entity: fact.entity.clone(),
            evidence: fact.evidence.clone(),
            retrieval_score: normalize_score(&candidate)?,
            estimated_tokens: fact.estimated_tokens,
            sources: candidate.sources.into_iter().map(map_source).collect(),
            embedding: candidate
                .embedding
                .map(|metadata| KnowledgeEmbeddingProvenance {
                    model_version: metadata.model_version,
                    dimensions: metadata.dimensions,
                    source_hash: metadata.source_hash,
                    chunk_version: metadata.chunk_version,
                }),
        });
    }
    let projection = KnowledgeRetrievalProjection {
        contract_version: KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION.into(),
        retrieval_version: HYBRID_RETRIEVAL_VERSION.into(),
        candidates,
        omitted,
        notice: KnowledgeRetrievalNotice {
            vector_status: match result.notice.vector_status {
                RetrievalAvailability::Available => KnowledgeRetrievalAvailability::Available,
                RetrievalAvailability::Unavailable => KnowledgeRetrievalAvailability::Unavailable,
            },
            used_fallback: result.notice.used_fallback,
            safe_message: result.notice.safe_message,
        },
    };
    projection
        .validate()
        .map_err(|error| RetrievalProjectionError::InvalidProjection(error.to_string()))?;
    Ok(projection)
}

fn normalize_score(candidate: &RetrievalCandidate) -> Result<i64, RetrievalProjectionError> {
    if !candidate.score.is_finite() {
        return Err(RetrievalProjectionError::InvalidScore {
            id: candidate.id.clone(),
        });
    }
    let scaled = (f64::from(candidate.score) * 1_000_000.0).round();
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(RetrievalProjectionError::InvalidScore {
            id: candidate.id.clone(),
        });
    }
    Ok(scaled as i64)
}

fn map_source(source: RetrievalSource) -> KnowledgeRetrievalSource {
    match source {
        RetrievalSource::Vector => KnowledgeRetrievalSource::Vector,
        RetrievalSource::FullText => KnowledgeRetrievalSource::FullText,
        RetrievalSource::Relation => KnowledgeRetrievalSource::Relation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::provider::{EmbeddingMetadata, RetrievalNotice},
        domain::contracts::{
            EvidenceRef, EvidenceTarget, GeneratorKind, GeneratorRef, KNOWLEDGE_CONTRACT_VERSION,
            KnowledgeEntityKind, KnowledgeScope, KnowledgeStatus, ScopedEvidenceRef,
        },
    };

    fn entity(id: &str) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: id.into(),
            kind: KnowledgeEntityKind::Decision,
            name: format!("Decision {id}"),
            aliases: vec![],
            scope: KnowledgeScope::Workspace {
                workspace_id: "workspace".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: GeneratorRef {
                kind: GeneratorKind::User,
                generator_id: "user".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-27T00:00:00Z".into(),
            updated_at: "2026-08-27T00:00:00Z".into(),
        }
    }

    fn result(score: f32) -> RetrievalResult {
        RetrievalResult {
            candidates: vec![RetrievalCandidate {
                id: "entity-1".into(),
                score,
                sources: vec![RetrievalSource::Vector, RetrievalSource::FullText],
                embedding: Some(EmbeddingMetadata {
                    model_version: "local-v1".into(),
                    dimensions: 32,
                    source_hash: "source-hash".into(),
                    chunk_version: "chunk-v1".into(),
                }),
            }],
            notice: RetrievalNotice {
                vector_status: RetrievalAvailability::Available,
                used_fallback: false,
                safe_message: None,
            },
        }
    }

    #[test]
    fn projects_hydrated_hybrid_candidates_with_sources_and_embedding_provenance() {
        let hydrated = HashMap::from([(
            "entity-1".into(),
            HydratedRetrievalCandidate {
                entity: entity("entity-1"),
                evidence: vec![],
                estimated_tokens: 8,
            },
        )]);
        let projection = project_retrieval_result(result(0.75), &hydrated).expect("projection");
        assert_eq!(projection.candidates[0].retrieval_score, 750_000);
        assert_eq!(projection.candidates[0].sources.len(), 2);
        assert_eq!(
            projection.candidates[0]
                .embedding
                .as_ref()
                .expect("embedding")
                .source_hash,
            "source-hash"
        );
    }

    #[test]
    fn omits_index_hits_missing_from_the_validated_fact_source() {
        let projection =
            project_retrieval_result(result(0.5), &HashMap::new()).expect("projection");
        assert!(projection.candidates.is_empty());
        assert_eq!(projection.omitted[0].reference_id, "entity-1");
    }

    #[test]
    fn rejects_non_finite_scores_before_they_cross_the_domain_boundary() {
        let hydrated = HashMap::from([(
            "entity-1".into(),
            HydratedRetrievalCandidate {
                entity: entity("entity-1"),
                evidence: vec![],
                estimated_tokens: 8,
            },
        )]);
        assert!(matches!(
            project_retrieval_result(result(f32::NAN), &hydrated),
            Err(RetrievalProjectionError::InvalidScore { .. })
        ));
    }

    #[test]
    fn semantic_vector_recall_returns_the_confirmed_evidence_ref() {
        let mut rain = entity("rain-guidance");
        rain.name = "出门前请携带雨具，以免被淋湿".into();
        rain.evidence.push(ScopedEvidenceRef {
            id: "scoped-rain".into(),
            evidence: EvidenceRef {
                id: "evidence-rain".into(),
                target: EvidenceTarget::ImportContent {
                    import_source_id: "source-weather".into(),
                    import_revision_id: "revision-weather".into(),
                    locator: "message-rain".into(),
                },
                content_hash: Some("sha256:rain".into()),
                excerpt: Some("明天有雨，记得带伞".into()),
                created_at: "2026-08-30T00:00:00Z".into(),
            },
            scope: rain.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: rain.generator.clone(),
        });
        let mut database = entity("database-note");
        database.name = "SQLite 是一种嵌入式关系数据库".into();
        let dimensions = 384;
        let mut rain_vector = vec![0.0; dimensions];
        rain_vector[0] = 1.0;
        let mut database_vector = vec![0.0; dimensions];
        database_vector[1] = 1.0;
        let records = vec![
            build_knowledge_embedding_record_from_vector(
                &rain,
                "semantic-test-v1",
                dimensions,
                rain_vector.clone(),
            )
            .expect("rain record"),
            build_knowledge_embedding_record_from_vector(
                &database,
                "semantic-test-v1",
                dimensions,
                database_vector,
            )
            .expect("database record"),
        ];

        let projection = retrieve_validated_knowledge_with_semantic(
            "conversation",
            "外面下雨需要带伞吗",
            1,
            &[rain, database],
            &[],
            vec![],
            KnowledgeVectorSnapshot {
                availability: RetrievalAvailability::Available,
                records,
            },
            Some(&SemanticQueryEmbedding {
                model_version: "semantic-test-v1".into(),
                dimensions,
                vector: rain_vector,
            }),
        )
        .expect("semantic retrieval projection");

        assert_eq!(projection.candidates[0].entity.id, "rain-guidance");
        assert_eq!(projection.candidates[0].evidence[0].id, "evidence-rain");
        assert_eq!(
            projection.candidates[0]
                .embedding
                .as_ref()
                .expect("semantic provenance")
                .dimensions,
            384
        );
    }
}
