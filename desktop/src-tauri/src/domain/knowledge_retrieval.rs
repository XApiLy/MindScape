use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    KernelError, KernelResult, OmittedKnowledgeRef,
    contracts::{EvidenceRef, KnowledgeEntity},
};

pub const KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION: &str =
    "mindscape.knowledge-retrieval.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeRetrievalSource {
    Vector,
    FullText,
    Relation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeRetrievalAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRetrievalNotice {
    pub vector_status: KnowledgeRetrievalAvailability,
    pub used_fallback: bool,
    pub safe_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEmbeddingProvenance {
    pub model_version: String,
    pub dimensions: usize,
    pub source_hash: String,
    pub chunk_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRetrievalCandidateProjection {
    pub entity: KnowledgeEntity,
    pub evidence: Vec<EvidenceRef>,
    pub retrieval_score: i64,
    pub estimated_tokens: i64,
    pub sources: Vec<KnowledgeRetrievalSource>,
    pub embedding: Option<KnowledgeEmbeddingProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRetrievalProjection {
    pub contract_version: String,
    pub retrieval_version: String,
    pub candidates: Vec<KnowledgeRetrievalCandidateProjection>,
    pub omitted: Vec<OmittedKnowledgeRef>,
    pub notice: KnowledgeRetrievalNotice,
}

impl KnowledgeRetrievalProjection {
    /// Validate the safe, hydrated boundary shared by FTS, vector and
    /// relation adapters. This type is a candidate projection only; the
    /// FocusFrame compiler remains the authority that decides what can enter
    /// a model context.
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported KnowledgeRetrieval projection contract version: {}",
                self.contract_version
            )));
        }
        if self.retrieval_version.trim().is_empty() {
            return Err(KernelError::Validation(
                "KnowledgeRetrieval retrieval version must not be empty".into(),
            ));
        }
        validate_notice(&self.notice)?;

        let mut ids = HashSet::new();
        for candidate in &self.candidates {
            candidate.entity.validate()?;
            if candidate.estimated_tokens <= 0 {
                return Err(KernelError::Validation(format!(
                    "KnowledgeRetrieval candidate {} must have a positive token estimate",
                    candidate.entity.id
                )));
            }
            if candidate.sources.is_empty() {
                return Err(KernelError::Validation(format!(
                    "KnowledgeRetrieval candidate {} must identify at least one source",
                    candidate.entity.id
                )));
            }
            if let Some(embedding) = &candidate.embedding
                && (embedding.model_version.trim().is_empty()
                    || embedding.source_hash.trim().is_empty()
                    || embedding.chunk_version.trim().is_empty()
                    || embedding.dimensions == 0)
            {
                return Err(KernelError::Validation(format!(
                    "KnowledgeRetrieval candidate {} has incomplete embedding provenance",
                    candidate.entity.id
                )));
            }
            let mut sources = HashSet::new();
            for source in &candidate.sources {
                if !sources.insert(*source) {
                    return Err(KernelError::Validation(format!(
                        "KnowledgeRetrieval candidate {} contains duplicate sources",
                        candidate.entity.id
                    )));
                }
            }
            for evidence in &candidate.evidence {
                evidence.validate()?;
            }
            if !ids.insert(candidate.entity.id.as_str()) {
                return Err(KernelError::Validation(format!(
                    "KnowledgeRetrieval candidate appears more than once: {}",
                    candidate.entity.id
                )));
            }
        }
        for omitted in &self.omitted {
            if omitted.reference_id.trim().is_empty() || omitted.reason.trim().is_empty() {
                return Err(KernelError::Validation(
                    "KnowledgeRetrieval omitted references require an id and reason".into(),
                ));
            }
            if !ids.insert(omitted.reference_id.as_str()) {
                return Err(KernelError::Validation(format!(
                    "KnowledgeRetrieval reference appears in both candidates and omitted: {}",
                    omitted.reference_id
                )));
            }
        }
        Ok(())
    }
}

fn validate_notice(notice: &KnowledgeRetrievalNotice) -> KernelResult<()> {
    if notice.used_fallback && notice.vector_status == KnowledgeRetrievalAvailability::Available {
        return Err(KernelError::Integrity(
            "KnowledgeRetrieval fallback cannot be marked when vector retrieval is available"
                .into(),
        ));
    }
    if notice
        .safe_message
        .as_deref()
        .is_some_and(|message| message.trim().is_empty())
    {
        return Err(KernelError::Validation(
            "KnowledgeRetrieval safe message must not be empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{
        GeneratorKind, GeneratorRef, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntityKind,
        KnowledgeScope, KnowledgeStatus,
    };

    fn entity(id: &str) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: id.into(),
            kind: KnowledgeEntityKind::Decision,
            name: format!("Decision {id}"),
            aliases: vec![],
            scope: KnowledgeScope::Workspace {
                workspace_id: "workspace-1".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: GeneratorRef {
                kind: GeneratorKind::User,
                generator_id: "user".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    fn projection() -> KnowledgeRetrievalProjection {
        KnowledgeRetrievalProjection {
            contract_version: KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION.into(),
            retrieval_version: "fts-v1+vector-v1".into(),
            candidates: vec![KnowledgeRetrievalCandidateProjection {
                entity: entity("entity-1"),
                evidence: vec![],
                retrieval_score: 42,
                estimated_tokens: 4,
                sources: vec![KnowledgeRetrievalSource::FullText],
                embedding: None,
            }],
            omitted: vec![OmittedKnowledgeRef {
                reference_id: "entity-2".into(),
                reason: "outside FocusFrame scope".into(),
            }],
            notice: KnowledgeRetrievalNotice {
                vector_status: KnowledgeRetrievalAvailability::Available,
                used_fallback: false,
                safe_message: None,
            },
        }
    }

    #[test]
    fn validates_hydrated_candidate_projection_and_notice() {
        projection().validate().expect("valid retrieval projection");
    }

    #[test]
    fn rejects_duplicate_candidate_ids_and_empty_sources() {
        let mut invalid = projection();
        invalid.candidates.push(invalid.candidates[0].clone());
        let error = invalid.validate().expect_err("duplicate candidate");
        assert!(error.to_string().contains("appears more than once"));

        let mut invalid = projection();
        invalid.candidates[0].sources.clear();
        let error = invalid.validate().expect_err("missing source provenance");
        assert!(error.to_string().contains("at least one source"));
    }

    #[test]
    fn rejects_inconsistent_vector_fallback_notice() {
        let mut invalid = projection();
        invalid.notice.used_fallback = true;
        let error = invalid
            .validate()
            .expect_err("inconsistent fallback notice");
        assert!(error.to_string().contains("fallback cannot be marked"));
    }

    #[test]
    fn rejects_incomplete_embedding_provenance() {
        let mut invalid = projection();
        invalid.candidates[0].embedding = Some(KnowledgeEmbeddingProvenance {
            model_version: "local-v1".into(),
            dimensions: 0,
            source_hash: "hash".into(),
            chunk_version: "chunk-v1".into(),
        });
        let error = invalid
            .validate()
            .expect_err("incomplete embedding metadata");
        assert!(
            error
                .to_string()
                .contains("incomplete embedding provenance")
        );
    }
}
