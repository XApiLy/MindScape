use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const LOCAL_EMBEDDING_MODEL_VERSION: &str = "mindscape-local-hash-embedding-v1";
pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingMetadata {
    pub model_version: String,
    pub dimensions: usize,
    pub source_hash: String,
    pub chunk_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingRecord {
    pub id: String,
    pub metadata: EmbeddingMetadata,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VectorMatch {
    pub id: String,
    pub score: f32,
    pub metadata: EmbeddingMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RetrievalSource {
    Vector,
    FullText,
    Relation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalCandidate {
    pub id: String,
    pub score: f32,
    pub sources: Vec<RetrievalSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<EmbeddingMetadata>,
}

impl From<&VectorMatch> for RetrievalCandidate {
    fn from(value: &VectorMatch) -> Self {
        Self {
            id: value.id.clone(),
            score: value.score,
            sources: vec![RetrievalSource::Vector],
            embedding: Some(value.metadata.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RetrievalAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalNotice {
    pub vector_status: RetrievalAvailability,
    pub used_fallback: bool,
    pub safe_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalResult {
    pub candidates: Vec<RetrievalCandidate>,
    pub notice: RetrievalNotice,
}

pub fn merge_retrieval_candidates(
    vector_status: RetrievalAvailability,
    vector: impl IntoIterator<Item = RetrievalCandidate>,
    full_text: impl IntoIterator<Item = RetrievalCandidate>,
    relations: impl IntoIterator<Item = RetrievalCandidate>,
    limit: usize,
) -> RetrievalResult {
    let vector_available = vector_status == RetrievalAvailability::Available;
    let mut merged = Vec::<RetrievalCandidate>::new();
    let mut indexes = std::collections::HashMap::<String, usize>::new();
    if vector_available {
        append_retrieval_candidates(&mut merged, &mut indexes, vector);
    }
    let mut fallback_items = full_text.into_iter().chain(relations).peekable();
    let used_fallback = !vector_available && fallback_items.peek().is_some();
    append_retrieval_candidates(&mut merged, &mut indexes, fallback_items);
    merged.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    merged.truncate(limit);
    RetrievalResult {
        candidates: merged,
        notice: RetrievalNotice {
            vector_status,
            used_fallback,
            safe_message: (!vector_available).then_some(
                "Vector retrieval is unavailable; using full-text and relation candidates.".into(),
            ),
        },
    }
}

fn append_retrieval_candidates(
    merged: &mut Vec<RetrievalCandidate>,
    indexes: &mut std::collections::HashMap<String, usize>,
    items: impl IntoIterator<Item = RetrievalCandidate>,
) {
    for item in items {
        if let Some(index) = indexes.get(&item.id).copied() {
            let existing = &mut merged[index];
            existing.score = existing.score.max(item.score);
            for source in item.sources {
                if !existing.sources.contains(&source) {
                    existing.sources.push(source);
                }
            }
            if existing.embedding.is_none() {
                existing.embedding = item.embedding;
            }
        } else {
            indexes.insert(item.id.clone(), merged.len());
            merged.push(item);
        }
    }
}

pub trait EmbeddingAdapter: Send + Sync {
    fn model_version(&self) -> &'static str;
    fn dimensions(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalHashEmbedding;

impl EmbeddingAdapter for LocalHashEmbedding {
    fn model_version(&self) -> &'static str {
        LOCAL_EMBEDDING_MODEL_VERSION
    }

    fn dimensions(&self) -> usize {
        DEFAULT_EMBEDDING_DIMENSIONS
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0_f32; self.dimensions()];
        for token in text.split_whitespace() {
            let digest = Sha256::digest(token.as_bytes());
            let bucket = u16::from_le_bytes([digest[0], digest[1]]) as usize % vector.len();
            let sign = if digest[2] & 1 == 0 { 1.0 } else { -1.0 };
            vector[bucket] += sign;
        }
        normalize(&mut vector);
        vector
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalVectorIndex {
    records: Vec<EmbeddingRecord>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VectorIndexRestoreError {
    #[error("vector record {id} uses model version {actual}, expected {expected}")]
    ModelVersionMismatch {
        id: String,
        actual: String,
        expected: String,
    },
    #[error("vector record {id} uses {actual} dimensions, expected {expected}")]
    DimensionMismatch {
        id: String,
        actual: usize,
        expected: usize,
    },
    #[error("vector record {id} has a zero or non-finite vector")]
    InvalidVector { id: String },
}

impl LocalVectorIndex {
    pub fn upsert_precomputed(
        &mut self,
        record: EmbeddingRecord,
    ) -> Result<(), VectorIndexRestoreError> {
        validate_record(
            &record,
            &record.metadata.model_version,
            record.metadata.dimensions,
        )?;
        if let Some(existing) = self.records.iter_mut().find(|item| item.id == record.id) {
            *existing = record;
        } else {
            self.records.push(record);
        }
        Ok(())
    }

    pub fn upsert<A: EmbeddingAdapter>(
        &mut self,
        adapter: &A,
        id: impl Into<String>,
        text: &str,
        source_hash: impl Into<String>,
        chunk_version: impl Into<String>,
    ) {
        let id = id.into();
        let record = EmbeddingRecord {
            id: id.clone(),
            metadata: EmbeddingMetadata {
                model_version: adapter.model_version().into(),
                dimensions: adapter.dimensions(),
                source_hash: source_hash.into(),
                chunk_version: chunk_version.into(),
            },
            vector: adapter.embed(text),
        };
        if let Some(existing) = self.records.iter_mut().find(|item| item.id == id) {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.records.len();
        self.records.retain(|record| record.id != id);
        self.records.len() != before
    }

    pub fn remove_source(&mut self, source_hash: &str) -> usize {
        let before = self.records.len();
        self.records
            .retain(|record| record.metadata.source_hash != source_hash);
        before - self.records.len()
    }

    pub fn remove_stale<A: EmbeddingAdapter>(&mut self, adapter: &A, chunk_version: &str) -> usize {
        let before = self.records.len();
        self.records.retain(|record| {
            record.metadata.model_version == adapter.model_version()
                && record.metadata.dimensions == adapter.dimensions()
                && record.vector.len() == adapter.dimensions()
                && record.metadata.chunk_version == chunk_version
        });
        before - self.records.len()
    }

    pub fn rebuild<A: EmbeddingAdapter>(&mut self, adapter: &A, items: &[IndexInput]) {
        self.records.clear();
        for item in items {
            self.upsert(
                adapter,
                item.id.clone(),
                &item.text,
                item.source_hash.clone(),
                item.chunk_version.clone(),
            );
        }
    }

    pub fn search<A: EmbeddingAdapter>(
        &self,
        adapter: &A,
        query: &str,
        limit: usize,
    ) -> Vec<VectorMatch> {
        if limit == 0 || self.records.is_empty() {
            return Vec::new();
        }
        let query = adapter.embed(query);
        self.search_precomputed(&query, limit)
    }

    pub fn search_precomputed(&self, query: &[f32], limit: usize) -> Vec<VectorMatch> {
        if limit == 0 || self.records.is_empty() || query.len() != self.records[0].vector.len() {
            return Vec::new();
        }
        let mut scored = self
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| (index, dot(query, &record.vector)))
            .collect::<Vec<_>>();
        let compare = |left: &(usize, f32), right: &(usize, f32)| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| self.records[left.0].id.cmp(&self.records[right.0].id))
        };
        if limit < scored.len() {
            scored.select_nth_unstable_by(limit, compare);
            scored.truncate(limit);
        }
        scored.sort_unstable_by(compare);
        scored
            .into_iter()
            .map(|(index, score)| {
                let record = &self.records[index];
                VectorMatch {
                    id: record.id.clone(),
                    score,
                    metadata: record.metadata.clone(),
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn snapshot(&self) -> Vec<EmbeddingRecord> {
        self.records.clone()
    }

    pub fn restore<A: EmbeddingAdapter>(
        adapter: &A,
        records: Vec<EmbeddingRecord>,
    ) -> Result<Self, VectorIndexRestoreError> {
        Self::restore_contract(adapter.model_version(), adapter.dimensions(), records)
    }

    pub fn restore_contract(
        model_version: &str,
        dimensions: usize,
        records: Vec<EmbeddingRecord>,
    ) -> Result<Self, VectorIndexRestoreError> {
        for record in &records {
            validate_record(record, model_version, dimensions)?;
        }
        Ok(Self { records })
    }
}

fn validate_record(
    record: &EmbeddingRecord,
    model_version: &str,
    dimensions: usize,
) -> Result<(), VectorIndexRestoreError> {
    if record.metadata.model_version != model_version {
        return Err(VectorIndexRestoreError::ModelVersionMismatch {
            id: record.id.clone(),
            actual: record.metadata.model_version.clone(),
            expected: model_version.into(),
        });
    }
    if record.metadata.dimensions != dimensions || record.vector.len() != dimensions {
        return Err(VectorIndexRestoreError::DimensionMismatch {
            id: record.id.clone(),
            actual: record.vector.len(),
            expected: dimensions,
        });
    }
    if record.vector.iter().any(|value| !value.is_finite())
        || record.vector.iter().all(|value| *value == 0.0)
    {
        return Err(VectorIndexRestoreError::InvalidVector {
            id: record.id.clone(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct IndexInput {
    pub id: String,
    pub text: String,
    pub source_hash: String,
    pub chunk_version: String,
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_embedding_is_deterministic_and_versioned() {
        let adapter = LocalHashEmbedding;
        assert_eq!(adapter.embed("alpha beta"), adapter.embed("alpha beta"));
        assert_eq!(adapter.dimensions(), DEFAULT_EMBEDDING_DIMENSIONS);
        assert_eq!(adapter.model_version(), LOCAL_EMBEDDING_MODEL_VERSION);
    }

    #[test]
    fn index_upserts_searches_removes_and_rebuilds_with_provenance() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        index.upsert(&adapter, "a", "sqlite local database", "hash-a", "chunk-v1");
        index.upsert(
            &adapter,
            "b",
            "postgres server database",
            "hash-b",
            "chunk-v1",
        );
        let matches = index.search(&adapter, "sqlite database", 1);
        assert_eq!(matches[0].id, "a");
        assert_eq!(matches[0].metadata.source_hash, "hash-a");
        assert!(index.remove("a"));
        assert_eq!(index.len(), 1);
        index.rebuild(
            &adapter,
            &[IndexInput {
                id: "c".into(),
                text: "vector rebuild source".into(),
                source_hash: "hash-c".into(),
                chunk_version: "chunk-v2".into(),
            }],
        );
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.search(&adapter, "vector", 1)[0]
                .metadata
                .chunk_version,
            "chunk-v2"
        );
    }

    #[test]
    fn upsert_replaces_stale_model_metadata_instead_of_merging_vectors() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        index.upsert(&adapter, "same", "old", "hash-old", "chunk-v1");
        index.upsert(&adapter, "same", "new", "hash-new", "chunk-v2");
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.search(&adapter, "new", 1)[0].metadata.source_hash,
            "hash-new"
        );
    }

    #[test]
    fn falls_back_to_full_text_and_relations_when_vector_is_unavailable() {
        let result = merge_retrieval_candidates(
            RetrievalAvailability::Unavailable,
            vec![RetrievalCandidate {
                id: "vector-only".into(),
                score: 1.0,
                sources: vec![RetrievalSource::Vector],
                embedding: None,
            }],
            vec![RetrievalCandidate {
                id: "entity-1".into(),
                score: 0.8,
                sources: vec![RetrievalSource::FullText],
                embedding: None,
            }],
            vec![RetrievalCandidate {
                id: "entity-1".into(),
                score: 0.7,
                sources: vec![RetrievalSource::Relation],
                embedding: None,
            }],
            10,
        );
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].sources,
            vec![RetrievalSource::FullText, RetrievalSource::Relation]
        );
        assert!(result.notice.used_fallback);
        assert!(result.notice.safe_message.is_some());
    }

    #[test]
    fn preserves_vector_priority_when_available() {
        let result = merge_retrieval_candidates(
            RetrievalAvailability::Available,
            vec![RetrievalCandidate {
                id: "entity-1".into(),
                score: 0.9,
                sources: vec![RetrievalSource::Vector],
                embedding: None,
            }],
            vec![RetrievalCandidate {
                id: "entity-1".into(),
                score: 0.8,
                sources: vec![RetrievalSource::FullText],
                embedding: None,
            }],
            Vec::new(),
            10,
        );
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].sources,
            vec![RetrievalSource::Vector, RetrievalSource::FullText]
        );
        assert!(!result.notice.used_fallback);
        assert!(result.notice.safe_message.is_none());
    }

    #[test]
    fn vector_match_conversion_preserves_embedding_provenance() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        index.upsert(
            &adapter,
            "entity",
            "local source",
            "source-hash",
            "chunk-v1",
        );
        let vector_match = index.search(&adapter, "local", 1).pop().expect("match");
        let candidate = RetrievalCandidate::from(&vector_match);
        assert_eq!(candidate.sources, vec![RetrievalSource::Vector]);
        assert_eq!(
            candidate.embedding.expect("metadata").source_hash,
            "source-hash"
        );
    }

    #[test]
    fn snapshots_and_restores_only_records_for_the_active_embedding_contract() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        index.upsert(&adapter, "a", "sqlite database", "hash-a", "chunk-v1");
        let restored = LocalVectorIndex::restore(&adapter, index.snapshot()).expect("restore");
        assert_eq!(restored.search(&adapter, "sqlite", 1)[0].id, "a");

        let mut invalid = restored.snapshot();
        invalid[0].metadata.model_version = "other-model".into();
        assert!(matches!(
            LocalVectorIndex::restore(&adapter, invalid),
            Err(VectorIndexRestoreError::ModelVersionMismatch { .. })
        ));
    }

    #[test]
    fn search_selects_a_stable_top_k_without_materializing_every_match() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        for item in 0..1_000 {
            index.upsert(
                &adapter,
                format!("entity-{item:04}"),
                &format!("shared token-{item}"),
                format!("hash-{item}"),
                "chunk-v1",
            );
        }

        let top_ten = index.search(&adapter, "shared token-42", 10);
        let all = index.search(&adapter, "shared token-42", index.len());

        assert_eq!(top_ten, all.into_iter().take(10).collect::<Vec<_>>());
        assert!(index.search(&adapter, "shared", 0).is_empty());
    }

    #[test]
    fn source_deletion_and_version_changes_invalidate_derived_vectors() {
        let adapter = LocalHashEmbedding;
        let mut index = LocalVectorIndex::default();
        index.upsert(&adapter, "a-1", "first", "source-a", "chunk-v1");
        index.upsert(&adapter, "a-2", "second", "source-a", "chunk-v1");
        index.upsert(&adapter, "b-1", "third", "source-b", "chunk-v2");

        assert_eq!(index.remove_source("source-a"), 2);
        assert_eq!(index.len(), 1);
        assert_eq!(index.remove_stale(&adapter, "chunk-v3"), 1);
        assert!(index.is_empty());
    }
}
