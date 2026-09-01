use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::io;
#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::{
    FocusPromotionDecisionPlan, FocusPromotionEntityMutation, KernelError, KernelResult,
    contracts::{
        DiscussionLog, DiscussionLogProjection, EvidenceRef, EvidenceTarget, KnowledgeEntity,
        KnowledgeRelation,
    },
};

const MANAGED_LINKS_MARKER: &str = "<!-- mindscape:managed-links -->";
const MAX_MARKDOWN_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct MarkdownVault {
    root: PathBuf,
    #[cfg(test)]
    fail_next_discussion_index_write: Arc<AtomicBool>,
    #[cfg(test)]
    fail_next_entity_index_write: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownEntityEdit {
    pub relative_path: String,
    pub content_hash: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDiscussionEdit {
    pub relative_path: String,
    pub content_hash: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug)]
pub struct MarkdownVaultMutationBackup {
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    journal_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FocusPromotionVaultJournal {
    decision_id: String,
    entries: Vec<FocusPromotionVaultJournalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FocusPromotionVaultJournalEntry {
    relative_path: String,
    backup_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionVaultJournal {
    discussion_log_id: String,
    revision: u64,
    entries: Vec<FocusPromotionVaultJournalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeDeleteVaultJournal {
    entity_id: String,
    neighbor_entity_ids: Vec<String>,
    entries: Vec<FocusPromotionVaultJournalEntry>,
}

impl MarkdownVault {
    pub fn new(root: impl AsRef<Path>) -> KernelResult<Self> {
        let root = root.as_ref().to_path_buf();
        for relative in [
            "entities",
            "sources",
            "logs/discussions",
            "indexes",
            ".transactions",
            ".discussion-transactions",
            ".entity-delete-transactions",
        ] {
            fs::create_dir_all(root.join(relative))?;
        }
        Ok(Self {
            root,
            #[cfg(test)]
            fail_next_discussion_index_write: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            fail_next_entity_index_write: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn write_entity(&self, entity: &KnowledgeEntity) -> KernelResult<(String, String)> {
        self.write_entity_with_relations(entity, &[])
    }

    pub fn write_entity_with_relations(
        &self,
        entity: &KnowledgeEntity,
        relations: &[KnowledgeRelation],
    ) -> KernelResult<(String, String)> {
        validate_stable_id(&entity.id)?;
        entity.validate()?;
        for scoped in &entity.evidence {
            self.write_evidence_source(&scoped.evidence)?;
        }
        let relative_path = format!("entities/{}.md", entity.id);
        let destination = self.root.join("entities").join(format!("{}.md", entity.id));
        let content = render_entity(entity, relations)?;
        let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        replace_file(&destination, content.as_bytes())?;
        Ok((relative_path, content_hash))
    }

    pub fn write_entity_index(&self, entities: &[KnowledgeEntity]) -> KernelResult<()> {
        #[cfg(test)]
        if self
            .fail_next_entity_index_write
            .swap(false, Ordering::AcqRel)
        {
            return Err(io::Error::other("injected entity index write failure").into());
        }
        let mut content = String::from("# Knowledge entities\n\n");
        for entity in entities {
            validate_stable_id(&entity.id)?;
            content.push_str(&format!(
                "- [[../entities/{}|{}]] — `{:?}` / `{:?}`\n",
                entity.id,
                markdown_inline(&entity.name),
                entity.kind,
                entity.status
            ));
        }
        replace_file(&self.root.join("indexes/entities.md"), content.as_bytes())
    }

    #[cfg(test)]
    pub fn inject_next_entity_index_write_failure(&self) {
        self.fail_next_entity_index_write
            .store(true, Ordering::Release);
    }

    /// Applies the Vault side of a focus decision while retaining enough state
    /// to compensate if the SQLite transaction rejects a concurrent version.
    pub fn apply_focus_promotion_plan(
        &self,
        plan: &FocusPromotionDecisionPlan,
        final_entities: &[KnowledgeEntity],
        relations: &[KnowledgeRelation],
    ) -> KernelResult<MarkdownVaultMutationBackup> {
        let projected = match &plan.entity_mutation {
            FocusPromotionEntityMutation::UpsertSource(source) => vec![source.as_ref()],
            FocusPromotionEntityMutation::Promote { source, promoted } => {
                vec![source.as_ref(), promoted.as_ref()]
            }
            FocusPromotionEntityMutation::DeleteSource { entity_id, .. } => {
                let neighbor_ids = relations
                    .iter()
                    .filter_map(|relation| {
                        if relation.source_entity_id == *entity_id {
                            Some(relation.target_entity_id.as_str())
                        } else if relation.target_entity_id == *entity_id {
                            Some(relation.source_entity_id.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<_>>();
                let neighbors = final_entities
                    .iter()
                    .filter(|entity| neighbor_ids.contains(entity.id.as_str()))
                    .collect::<Vec<_>>();
                if neighbors.len() != neighbor_ids.len() {
                    return Err(KernelError::Integrity(
                        "focus promotion delete relation references a missing neighbor entity"
                            .into(),
                    ));
                }
                neighbors
            }
        };
        let final_relations = match &plan.entity_mutation {
            FocusPromotionEntityMutation::DeleteSource { entity_id, .. } => relations
                .iter()
                .filter(|relation| {
                    relation.source_entity_id != *entity_id
                        && relation.target_entity_id != *entity_id
                })
                .cloned()
                .collect::<Vec<_>>(),
            _ => relations.to_vec(),
        };
        let mut paths = vec![self.root.join("indexes/entities.md")];
        for entity in &projected {
            validate_stable_id(&entity.id)?;
            paths.push(self.root.join("entities").join(format!("{}.md", entity.id)));
            for evidence in &entity.evidence {
                validate_stable_id(&evidence.evidence.id)?;
                paths.push(
                    self.root
                        .join("sources")
                        .join(format!("{}.md", evidence.evidence.id)),
                );
            }
        }
        if let FocusPromotionEntityMutation::DeleteSource { entity_id, .. } = &plan.entity_mutation
        {
            validate_stable_id(entity_id)?;
            paths.push(self.root.join("entities").join(format!("{entity_id}.md")));
        }
        paths.sort();
        paths.dedup();
        let files = paths
            .into_iter()
            .map(|path| {
                let content = if path.exists() {
                    Some(fs::read(&path)?)
                } else {
                    None
                };
                Ok((path, content))
            })
            .collect::<KernelResult<Vec<_>>>()?;
        let journal_key = format!("{:x}", Sha256::digest(plan.decision.decision_id.as_bytes()));
        let journal_dir = self.root.join(".transactions").join(journal_key);
        if journal_dir.exists() {
            return Err(KernelError::Integrity(format!(
                "Vault transaction for focus decision {} is already pending",
                plan.decision.decision_id
            )));
        }
        fs::create_dir(&journal_dir)?;
        let journal = FocusPromotionVaultJournal {
            decision_id: plan.decision.decision_id.clone(),
            entries: files
                .iter()
                .enumerate()
                .map(|(index, (path, content))| {
                    let relative_path = path
                        .strip_prefix(&self.root)
                        .map_err(|_| {
                            KernelError::Integrity(
                                "Vault transaction path escaped the Vault root".into(),
                            )
                        })?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let backup_file = content.as_ref().map(|_| format!("{index}.bak"));
                    if let (Some(content), Some(backup_file)) = (content, backup_file.as_ref()) {
                        write_new_synced(&journal_dir.join(backup_file), content)?;
                    }
                    Ok(FocusPromotionVaultJournalEntry {
                        relative_path,
                        backup_file,
                    })
                })
                .collect::<KernelResult<Vec<_>>>()?,
        };
        write_new_synced(
            &journal_dir.join("manifest.json"),
            &serde_json::to_vec(&journal)?,
        )?;
        let backup = MarkdownVaultMutationBackup { files, journal_dir };

        let result = (|| {
            match &plan.entity_mutation {
                FocusPromotionEntityMutation::UpsertSource(source) => {
                    self.write_entity_with_relations(source, &final_relations)?;
                }
                FocusPromotionEntityMutation::Promote { source, promoted } => {
                    self.write_entity_with_relations(source, &final_relations)?;
                    self.write_entity_with_relations(promoted, &final_relations)?;
                }
                FocusPromotionEntityMutation::DeleteSource { entity_id, .. } => {
                    self.remove_entity(entity_id)?;
                    for entity in &projected {
                        self.write_entity_with_relations(entity, &final_relations)?;
                    }
                }
            }
            self.write_entity_index(final_entities)
        })();
        if let Err(error) = result {
            if let Err(rollback_error) = self.rollback_focus_promotion(backup) {
                return Err(KernelError::Integrity(format!(
                    "Vault focus promotion failed ({error}); rollback also failed ({rollback_error})"
                )));
            }
            return Err(error);
        }
        Ok(backup)
    }

    pub fn rollback_focus_promotion(
        &self,
        backup: MarkdownVaultMutationBackup,
    ) -> KernelResult<()> {
        self.rollback_mutation(backup)
    }

    pub fn commit_focus_promotion(&self, backup: MarkdownVaultMutationBackup) -> KernelResult<()> {
        self.commit_mutation(backup)
    }

    fn rollback_mutation(&self, backup: MarkdownVaultMutationBackup) -> KernelResult<()> {
        for (path, content) in backup.files.into_iter().rev() {
            match content {
                Some(content) => replace_file(&path, &content)?,
                None if path.exists() => fs::remove_file(path)?,
                None => {}
            }
        }
        if backup.journal_dir.exists() {
            fs::remove_dir_all(backup.journal_dir)?;
        }
        Ok(())
    }

    fn commit_mutation(&self, backup: MarkdownVaultMutationBackup) -> KernelResult<()> {
        if backup.journal_dir.exists() {
            fs::remove_dir_all(backup.journal_dir)?;
        }
        Ok(())
    }

    pub fn apply_discussion_log(
        &self,
        log: &DiscussionLog,
    ) -> KernelResult<(String, String, MarkdownVaultMutationBackup)> {
        validate_stable_id(&log.id)?;
        log.validate()?;
        let mut paths = vec![
            self.root
                .join("logs/discussions")
                .join(format!("{}.md", log.id)),
            self.root.join("indexes/discussions.md"),
        ];
        for evidence in &log.evidence {
            validate_stable_id(&evidence.id)?;
            paths.push(
                self.root
                    .join("sources")
                    .join(format!("{}.md", evidence.id)),
            );
        }
        paths.sort();
        paths.dedup();
        let files = paths
            .into_iter()
            .map(|path| {
                let content = path.is_file().then(|| fs::read(&path)).transpose()?;
                Ok((path, content))
            })
            .collect::<KernelResult<Vec<_>>>()?;
        let journal_key = format!(
            "{:x}",
            Sha256::digest(format!("{}:{}", log.id, log.revision).as_bytes())
        );
        let journal_dir = self.root.join(".discussion-transactions").join(journal_key);
        if journal_dir.exists() {
            return Err(KernelError::Integrity(format!(
                "Vault transaction for DiscussionLog {} revision {} is already pending",
                log.id, log.revision
            )));
        }
        fs::create_dir(&journal_dir)?;
        let journal = DiscussionVaultJournal {
            discussion_log_id: log.id.clone(),
            revision: log.revision,
            entries: files
                .iter()
                .enumerate()
                .map(|(index, (path, content))| {
                    let relative_path = path
                        .strip_prefix(&self.root)
                        .map_err(|_| {
                            KernelError::Integrity(
                                "DiscussionLog transaction path escaped the Vault root".into(),
                            )
                        })?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let backup_file = content.as_ref().map(|_| format!("{index}.bak"));
                    if let (Some(content), Some(backup_file)) = (content, backup_file.as_ref()) {
                        write_new_synced(&journal_dir.join(backup_file), content)?;
                    }
                    Ok(FocusPromotionVaultJournalEntry {
                        relative_path,
                        backup_file,
                    })
                })
                .collect::<KernelResult<Vec<_>>>()?,
        };
        write_new_synced(
            &journal_dir.join("manifest.json"),
            &serde_json::to_vec(&journal)?,
        )?;
        let backup = MarkdownVaultMutationBackup { files, journal_dir };
        match self.write_discussion_log(log) {
            Ok((relative_path, content_hash)) => Ok((relative_path, content_hash, backup)),
            Err(error) => match self.rollback_mutation(backup) {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(KernelError::Integrity(format!(
                    "DiscussionLog Vault projection failed ({error}); rollback also failed ({rollback_error})"
                ))),
            },
        }
    }

    pub fn rollback_discussion_log(&self, backup: MarkdownVaultMutationBackup) -> KernelResult<()> {
        self.rollback_mutation(backup)
    }

    pub fn commit_discussion_log(&self, backup: MarkdownVaultMutationBackup) -> KernelResult<()> {
        self.commit_mutation(backup)
    }

    pub fn recover_discussion_transactions(
        &self,
        projections: &[DiscussionLogProjection],
    ) -> KernelResult<u64> {
        let committed =
            projections
                .iter()
                .fold(HashMap::<&str, u64>::new(), |mut revisions, projection| {
                    revisions
                        .entry(&projection.log.id)
                        .and_modify(|revision| *revision = (*revision).max(projection.log.revision))
                        .or_insert(projection.log.revision);
                    revisions
                });
        let transactions = self.root.join(".discussion-transactions");
        let mut recovered = 0;
        let mut committed_journals = Vec::new();
        for entry in fs::read_dir(&transactions)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let journal_dir = entry.path();
            let manifest_path = journal_dir.join("manifest.json");
            if !manifest_path.is_file() {
                fs::remove_dir_all(journal_dir)?;
                recovered += 1;
                continue;
            }
            let journal: DiscussionVaultJournal =
                serde_json::from_slice(&fs::read(manifest_path)?)?;
            if committed
                .get(journal.discussion_log_id.as_str())
                .is_some_and(|latest_revision| *latest_revision >= journal.revision)
            {
                committed_journals.push(journal_dir);
            } else {
                restore_journal_entries(&self.root, &journal_dir, journal.entries)?;
                fs::remove_dir_all(journal_dir)?;
            }
            recovered += 1;
        }
        self.write_discussion_index(projections)?;
        for journal_dir in committed_journals {
            fs::remove_dir_all(journal_dir)?;
        }
        Ok(recovered)
    }

    pub fn apply_knowledge_entity_delete(
        &self,
        entity_id: &str,
        neighbor_entities: &[&KnowledgeEntity],
        final_entities: &[KnowledgeEntity],
        final_relations: &[KnowledgeRelation],
    ) -> KernelResult<MarkdownVaultMutationBackup> {
        validate_stable_id(entity_id)?;
        let mut paths = vec![
            self.root.join("entities").join(format!("{entity_id}.md")),
            self.root.join("indexes/entities.md"),
        ];
        for entity in neighbor_entities {
            validate_stable_id(&entity.id)?;
            paths.push(self.root.join("entities").join(format!("{}.md", entity.id)));
            for evidence in &entity.evidence {
                validate_stable_id(&evidence.evidence.id)?;
                paths.push(
                    self.root
                        .join("sources")
                        .join(format!("{}.md", evidence.evidence.id)),
                );
            }
        }
        paths.sort();
        paths.dedup();
        let files = paths
            .into_iter()
            .map(|path| {
                let content = path.is_file().then(|| fs::read(&path)).transpose()?;
                Ok((path, content))
            })
            .collect::<KernelResult<Vec<_>>>()?;
        let journal_key = format!("{:x}", Sha256::digest(entity_id.as_bytes()));
        let journal_dir = self
            .root
            .join(".entity-delete-transactions")
            .join(journal_key);
        if journal_dir.exists() {
            return Err(KernelError::Integrity(format!(
                "Vault transaction for knowledge entity {entity_id} is already pending"
            )));
        }
        fs::create_dir(&journal_dir)?;
        let journal = KnowledgeDeleteVaultJournal {
            entity_id: entity_id.into(),
            neighbor_entity_ids: neighbor_entities
                .iter()
                .map(|entity| entity.id.clone())
                .collect(),
            entries: files
                .iter()
                .enumerate()
                .map(|(index, (path, content))| {
                    let relative_path = path
                        .strip_prefix(&self.root)
                        .map_err(|_| {
                            KernelError::Integrity(
                                "knowledge delete transaction path escaped the Vault root".into(),
                            )
                        })?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let backup_file = content.as_ref().map(|_| format!("{index}.bak"));
                    if let (Some(content), Some(backup_file)) = (content, backup_file.as_ref()) {
                        write_new_synced(&journal_dir.join(backup_file), content)?;
                    }
                    Ok(FocusPromotionVaultJournalEntry {
                        relative_path,
                        backup_file,
                    })
                })
                .collect::<KernelResult<Vec<_>>>()?,
        };
        write_new_synced(
            &journal_dir.join("manifest.json"),
            &serde_json::to_vec(&journal)?,
        )?;
        let backup = MarkdownVaultMutationBackup { files, journal_dir };
        let result = (|| {
            self.remove_entity(entity_id)?;
            for entity in neighbor_entities {
                self.write_entity_with_relations(entity, final_relations)?;
            }
            self.write_entity_index(final_entities)
        })();
        match result {
            Ok(()) => Ok(backup),
            Err(error) => match self.rollback_mutation(backup) {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(KernelError::Integrity(format!(
                    "knowledge delete Vault projection failed ({error}); rollback also failed ({rollback_error})"
                ))),
            },
        }
    }

    pub fn rollback_knowledge_entity_delete(
        &self,
        backup: MarkdownVaultMutationBackup,
    ) -> KernelResult<()> {
        self.rollback_mutation(backup)
    }

    pub fn commit_knowledge_entity_delete(
        &self,
        backup: MarkdownVaultMutationBackup,
    ) -> KernelResult<()> {
        self.commit_mutation(backup)
    }

    pub fn recover_knowledge_entity_delete_transactions(
        &self,
        entities: &[KnowledgeEntity],
        relations: &[KnowledgeRelation],
    ) -> KernelResult<u64> {
        let entity_ids = entities
            .iter()
            .map(|entity| entity.id.as_str())
            .collect::<HashSet<_>>();
        let transactions = self.root.join(".entity-delete-transactions");
        let mut recovered = 0;
        let mut committed_journals = Vec::new();
        for entry in fs::read_dir(&transactions)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let journal_dir = entry.path();
            let manifest_path = journal_dir.join("manifest.json");
            if !manifest_path.is_file() {
                fs::remove_dir_all(journal_dir)?;
                recovered += 1;
                continue;
            }
            let journal: KnowledgeDeleteVaultJournal =
                serde_json::from_slice(&fs::read(manifest_path)?)?;
            validate_stable_id(&journal.entity_id)?;
            for neighbor_id in &journal.neighbor_entity_ids {
                validate_stable_id(neighbor_id)?;
            }
            if entity_ids.contains(journal.entity_id.as_str()) {
                restore_journal_entries(&self.root, &journal_dir, journal.entries)?;
                fs::remove_dir_all(journal_dir)?;
            } else {
                self.remove_entity(&journal.entity_id)?;
                for entity in entities.iter().filter(|entity| {
                    journal
                        .neighbor_entity_ids
                        .iter()
                        .any(|neighbor_id| neighbor_id == &entity.id)
                }) {
                    self.write_entity_with_relations(entity, relations)?;
                }
                committed_journals.push(journal_dir);
            }
            recovered += 1;
        }
        self.write_entity_index(entities)?;
        for journal_dir in committed_journals {
            fs::remove_dir_all(journal_dir)?;
        }
        Ok(recovered)
    }

    /// Resolves a crash between Vault projection and SQLite commit. A
    /// committed decision keeps the projected files; an uncommitted decision
    /// restores every recorded pre-image.
    pub fn recover_focus_promotion_transactions(
        &self,
        committed_decision_ids: &HashSet<String>,
    ) -> KernelResult<u64> {
        let transactions = self.root.join(".transactions");
        let mut recovered = 0;
        for entry in fs::read_dir(&transactions)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let journal_dir = entry.path();
            let manifest_path = journal_dir.join("manifest.json");
            if !manifest_path.is_file() {
                fs::remove_dir_all(journal_dir)?;
                recovered += 1;
                continue;
            }
            let journal: FocusPromotionVaultJournal =
                serde_json::from_slice(&fs::read(manifest_path)?)?;
            if committed_decision_ids.contains(&journal.decision_id) {
                fs::remove_dir_all(journal_dir)?;
                recovered += 1;
                continue;
            }
            restore_journal_entries(&self.root, &journal_dir, journal.entries)?;
            fs::remove_dir_all(journal_dir)?;
            recovered += 1;
        }
        Ok(recovered)
    }

    pub fn write_discussion_log(&self, log: &DiscussionLog) -> KernelResult<(String, String)> {
        validate_stable_id(&log.id)?;
        log.validate()?;
        for evidence in &log.evidence {
            self.write_evidence_source(evidence)?;
        }
        for entity_id in &log.related_entity_ids {
            validate_stable_id(entity_id)?;
        }
        let relative_path = format!("logs/discussions/{}.md", log.id);
        let content = render_discussion_log(log)?;
        let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        replace_file(&self.root.join(&relative_path), content.as_bytes())?;
        Ok((relative_path, content_hash))
    }

    pub fn write_discussion_index(
        &self,
        projections: &[DiscussionLogProjection],
    ) -> KernelResult<()> {
        #[cfg(test)]
        if self
            .fail_next_discussion_index_write
            .swap(false, Ordering::AcqRel)
        {
            return Err(io::Error::other("injected DiscussionLog index write failure").into());
        }
        let mut content = String::from("# Discussion logs\n\n");
        for projection in projections {
            projection.validate()?;
            content.push_str(&format!(
                "- [[../logs/discussions/{}|{}]] — revision {}\n",
                projection.log.id,
                markdown_inline(&projection.log.title),
                projection.log.revision
            ));
        }
        replace_file(
            &self.root.join("indexes/discussions.md"),
            content.as_bytes(),
        )
    }

    #[cfg(test)]
    pub fn inject_next_discussion_index_write_failure(&self) {
        self.fail_next_discussion_index_write
            .store(true, Ordering::Release);
    }

    pub fn read_discussion_log_edit(
        &self,
        discussion_log_id: &str,
    ) -> KernelResult<MarkdownDiscussionEdit> {
        validate_stable_id(discussion_log_id)?;
        let relative_path = format!("logs/discussions/{discussion_log_id}.md");
        let content = fs::read_to_string(self.root.join(&relative_path))?;
        if content.len() > MAX_MARKDOWN_BYTES {
            return Err(KernelError::Validation(
                "Markdown discussion edit exceeds the 1 MiB limit".into(),
            ));
        }
        let persisted_id = frontmatter_value(&content, "mindscapeId")?;
        if persisted_id != discussion_log_id {
            return Err(KernelError::Integrity(
                "Markdown discussion stable ID does not match its Vault path".into(),
            ));
        }
        let document = markdown_body(&content)?;
        let (heading, after_heading) = document.split_once('\n').ok_or_else(|| {
            KernelError::Validation("Markdown discussion requires body content".into())
        })?;
        let title = heading
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                KernelError::Validation("Markdown discussion requires a level-one heading".into())
            })?;
        let (body_markdown, _) =
            after_heading
                .split_once(MANAGED_LINKS_MARKER)
                .ok_or_else(|| {
                    KernelError::Validation(
                        "Markdown discussion is missing its managed links boundary".into(),
                    )
                })?;
        let body_markdown = body_markdown.trim();
        if body_markdown.is_empty() {
            return Err(KernelError::Validation(
                "Markdown discussion requires an editable body".into(),
            ));
        }
        Ok(MarkdownDiscussionEdit {
            relative_path,
            content_hash: format!("{:x}", Sha256::digest(content.as_bytes())),
            title: title.into(),
            body_markdown: body_markdown.into(),
        })
    }

    pub fn remove_entity(&self, entity_id: &str) -> KernelResult<()> {
        validate_stable_id(entity_id)?;
        let path = self.root.join("entities").join(format!("{entity_id}.md"));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn remove_discussion_log(&self, discussion_log_id: &str) -> KernelResult<()> {
        validate_stable_id(discussion_log_id)?;
        let path = self
            .root
            .join("logs/discussions")
            .join(format!("{discussion_log_id}.md"));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn reconcile_entity_files(&self, entity_ids: &HashSet<String>) -> KernelResult<u64> {
        let mut removed = 0;
        for entry in fs::read_dir(self.root.join("entities"))? {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type()?.is_file()
                || path.extension().and_then(|v| v.to_str()) != Some("md")
            {
                continue;
            }
            let Some(entity_id) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            if validate_stable_id(entity_id).is_ok() && !entity_ids.contains(entity_id) {
                fs::remove_file(path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn recover_interrupted_writes(&self) -> KernelResult<u64> {
        let mut recovered = 0;
        for relative in ["entities", "sources", "logs/discussions", "indexes"] {
            for entry in fs::read_dir(self.root.join(relative))? {
                let path = entry?.path();
                let extension = path.extension().and_then(|value| value.to_str());
                if !matches!(extension, Some("next" | "previous")) {
                    continue;
                }
                let mut destination = path.clone();
                destination.set_extension("");
                if extension == Some("previous") && !destination.exists() {
                    fs::rename(&path, destination)?;
                } else {
                    fs::remove_file(path)?;
                }
                recovered += 1;
            }
        }
        Ok(recovered)
    }

    pub fn read_entity_edit(&self, entity_id: &str) -> KernelResult<MarkdownEntityEdit> {
        validate_stable_id(entity_id)?;
        let relative_path = format!("entities/{entity_id}.md");
        let content = fs::read_to_string(self.root.join(&relative_path))?;
        if content.len() > MAX_MARKDOWN_BYTES {
            return Err(KernelError::Validation(
                "Markdown entity edit exceeds the 1 MiB limit".into(),
            ));
        }
        let persisted_id = frontmatter_value(&content, "mindscapeId")?;
        if persisted_id != entity_id {
            return Err(KernelError::Integrity(
                "Markdown entity stable ID does not match its Vault path".into(),
            ));
        }
        let name = content
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                KernelError::Validation("Markdown entity requires a level-one heading".into())
            })?;
        Ok(MarkdownEntityEdit {
            relative_path,
            content_hash: format!("{:x}", Sha256::digest(content.as_bytes())),
            name: name.into(),
        })
    }
}

fn frontmatter_value(content: &str, key: &str) -> KernelResult<String> {
    let prefix = format!("{key}: ");
    let value = content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or_else(|| KernelError::Validation(format!("Markdown frontmatter is missing {key}")))?;
    serde_json::from_str(value).map_err(Into::into)
}

fn render_entity(
    entity: &KnowledgeEntity,
    relations: &[KnowledgeRelation],
) -> KernelResult<String> {
    let id = serde_json::to_string(&entity.id)?;
    let name = serde_json::to_string(&entity.name)?;
    let status = serde_json::to_string(&entity.status)?;
    let scope = serde_json::to_string(&entity.scope)?;
    let kind = serde_json::to_string(&entity.kind)?;
    let aliases = serde_json::to_string(&entity.aliases)?;
    let evidence_ids = serde_json::to_string(
        &entity
            .evidence
            .iter()
            .map(|reference| reference.evidence.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    let mut content = format!(
        "---\nmindscapeId: {id}\ncontractVersion: {}\nentityRevision: {}\nname: {name}\nkind: {kind}\nstatus: {status}\nscope: {scope}\naliases: {aliases}\nevidenceRefs: {evidence_ids}\n---\n\n# {}\n",
        entity.contract_version,
        entity.revision,
        markdown_inline(&entity.name)
    );
    if !entity.aliases.is_empty() {
        content.push_str("\n## Aliases\n\n");
        for alias in &entity.aliases {
            content.push_str(&format!("- {}\n", markdown_inline(alias)));
        }
    }
    content.push_str(&render_entity_relations(entity, relations)?);
    content.push_str(&render_evidence_links(
        entity.evidence.iter().map(|reference| &reference.evidence),
        "../sources",
    )?);
    Ok(content)
}

fn render_discussion_log(log: &DiscussionLog) -> KernelResult<String> {
    let id = serde_json::to_string(&log.id)?;
    let scope = serde_json::to_string(&log.scope)?;
    let evidence_ids = serde_json::to_string(
        &log.evidence
            .iter()
            .map(|evidence| evidence.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    let related_ids = serde_json::to_string(&log.related_entity_ids)?;
    let mut content = format!(
        "---\nmindscapeId: {id}\ncontractVersion: {}\nlogRevision: {}\nscope: {scope}\nevidenceRefs: {evidence_ids}\nrelatedEntities: {related_ids}\ncreatedAt: {}\nupdatedAt: {}\n---\n\n# {}\n\n{}\n\n{MANAGED_LINKS_MARKER}\n",
        log.contract_version,
        log.revision,
        serde_json::to_string(&log.created_at)?,
        serde_json::to_string(&log.updated_at)?,
        markdown_inline(&log.title),
        log.body_markdown.trim()
    );
    if !log.related_entity_ids.is_empty() {
        content.push_str("\n## Related knowledge\n\n");
        for entity_id in &log.related_entity_ids {
            validate_stable_id(entity_id)?;
            content.push_str(&format!("- [[../../entities/{entity_id}|{entity_id}]]\n"));
        }
    }
    content.push_str(&render_evidence_links(
        log.evidence.iter(),
        "../../sources",
    )?);
    Ok(content)
}

fn render_entity_relations(
    entity: &KnowledgeEntity,
    relations: &[KnowledgeRelation],
) -> KernelResult<String> {
    let mut content = String::new();
    for relation in relations {
        relation.validate()?;
        let related_id = if relation.source_entity_id == entity.id {
            Some(relation.target_entity_id.as_str())
        } else if relation.target_entity_id == entity.id {
            Some(relation.source_entity_id.as_str())
        } else {
            None
        };
        let Some(related_id) = related_id else {
            continue;
        };
        validate_stable_id(related_id)?;
        if content.is_empty() {
            content.push_str("\n## Related knowledge\n\n");
        }
        content.push_str(&format!(
            "- [[{related_id}|{related_id}]] — `{:?}` / `{:?}`\n",
            relation.kind, relation.status
        ));
    }
    Ok(content)
}

fn render_evidence_links<'a>(
    evidence: impl Iterator<Item = &'a EvidenceRef>,
    source_prefix: &str,
) -> KernelResult<String> {
    let mut content = String::new();
    for reference in evidence {
        reference.validate()?;
        validate_stable_id(&reference.id)?;
        if content.is_empty() {
            content.push_str("\n## Sources\n\n");
        }
        content.push_str(&format!(
            "- [[{}/{}|{}]] — `{}`",
            source_prefix,
            reference.id,
            reference.id,
            evidence_locator(&reference.target)
        ));
        if let Some(excerpt) = &reference.excerpt {
            content.push_str(&format!(" — {}", markdown_inline(excerpt)));
        }
        content.push('\n');
    }
    Ok(content)
}

impl MarkdownVault {
    fn write_evidence_source(&self, evidence: &EvidenceRef) -> KernelResult<()> {
        evidence.validate()?;
        validate_stable_id(&evidence.id)?;
        let id = serde_json::to_string(&evidence.id)?;
        let target = serde_json::to_string(&evidence.target)?;
        let mut content = format!(
            "---\nmindscapeId: {id}\ncontractVersion: mindscape.evidence.v1\ntarget: {target}\ncreatedAt: {}\n---\n\n# Source {}\n\nTarget: `{}`\n",
            serde_json::to_string(&evidence.created_at)?,
            evidence.id,
            evidence_locator(&evidence.target)
        );
        if let Some(content_hash) = &evidence.content_hash {
            content.push_str(&format!(
                "\nContent hash: `{}`\n",
                markdown_inline(content_hash)
            ));
        }
        if let Some(excerpt) = &evidence.excerpt {
            content.push_str("\n## Excerpt\n\n> ");
            content.push_str(&excerpt.replace('\n', "\n> "));
            content.push('\n');
        }
        replace_file(
            &self
                .root
                .join("sources")
                .join(format!("{}.md", evidence.id)),
            content.as_bytes(),
        )
    }
}

fn evidence_locator(target: &EvidenceTarget) -> String {
    match target {
        EvidenceTarget::MessageBlock {
            message_id,
            content_block_index,
        } => format!("message://{message_id}/block/{content_block_index}"),
        EvidenceTarget::ImportContent {
            import_source_id,
            import_revision_id,
            locator,
        } => format!("import://{import_source_id}/{import_revision_id}/{locator}"),
        EvidenceTarget::AttachmentContent {
            attachment_id,
            locator,
        } => format!(
            "attachment://{attachment_id}/{}",
            locator.as_deref().unwrap_or_default()
        ),
        EvidenceTarget::ToolResultBlock {
            tool_run_id,
            content_block_index,
        } => format!("tool://{tool_run_id}/block/{content_block_index}"),
    }
}

fn markdown_inline(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_owned()
}

fn markdown_body(content: &str) -> KernelResult<&str> {
    let after_start = content.strip_prefix("---\n").ok_or_else(|| {
        KernelError::Validation("Markdown document requires YAML frontmatter".into())
    })?;
    let (_, body) = after_start.split_once("\n---\n").ok_or_else(|| {
        KernelError::Validation("Markdown document frontmatter is not closed".into())
    })?;
    Ok(body.trim_start_matches('\n'))
}

fn write_new_synced(path: &Path, content: &[u8]) -> KernelResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}

fn safe_journal_destination(root: &Path, relative_path: &str) -> KernelResult<PathBuf> {
    let relative = Path::new(relative_path);
    let components = relative
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>();
    let allowed = components.as_ref().is_some_and(|components| {
        matches!(
            components.as_slice(),
            ["entities" | "sources" | "indexes", _]
        ) || matches!(components.as_slice(), ["logs", "discussions", _])
    });
    if !allowed || relative.is_absolute() {
        return Err(KernelError::Integrity(
            "Vault recovery journal contains an unsafe path".into(),
        ));
    }
    Ok(root.join(relative))
}

fn restore_journal_entries(
    root: &Path,
    journal_dir: &Path,
    entries: Vec<FocusPromotionVaultJournalEntry>,
) -> KernelResult<()> {
    for item in entries.into_iter().rev() {
        let destination = safe_journal_destination(root, &item.relative_path)?;
        match item.backup_file {
            Some(backup_file) => {
                let stem = backup_file.strip_suffix(".bak").unwrap_or_default();
                if stem.is_empty() || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(KernelError::Integrity(
                        "Vault recovery journal contains an unsafe backup path".into(),
                    ));
                }
                let content = fs::read(journal_dir.join(backup_file))?;
                replace_file(&destination, &content)?;
            }
            None if destination.exists() => fs::remove_file(destination)?,
            None => {}
        }
    }
    Ok(())
}

fn replace_file(destination: &Path, content: &[u8]) -> KernelResult<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| KernelError::Validation("Vault destination requires a parent".into()))?;
    fs::create_dir_all(parent)?;
    let next = destination.with_extension("md.next");
    let previous = destination.with_extension("md.previous");
    if next.exists() {
        fs::remove_file(&next)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&next)?;
    file.write_all(content)?;
    file.sync_all()?;
    drop(file);
    if previous.exists() {
        fs::remove_file(&previous)?;
    }
    if destination.exists() {
        fs::rename(destination, &previous)?;
    }
    if let Err(error) = fs::rename(&next, destination) {
        if previous.exists() {
            let _ = fs::rename(&previous, destination);
        }
        return Err(error.into());
    }
    if previous.exists() {
        fs::remove_file(previous)?;
    }
    Ok(())
}

fn validate_stable_id(id: &str) -> KernelResult<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(KernelError::Validation(
            "knowledge entity id is not safe for a Vault path".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::domain::contracts::{
        DISCUSSION_LOG_CONTRACT_VERSION, DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION,
        DiscussionLogScope, EvidenceRef, EvidenceTarget, GeneratorKind, GeneratorRef,
        KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntityKind, KnowledgeRelationKind, KnowledgeScope,
        KnowledgeStatus, ScopedEvidenceRef,
    };
    use crate::domain::{
        FOCUS_PROMOTION_DECISION_CONTRACT_VERSION, FocusPromotionDecisionAction,
        FocusPromotionDecisionPlan, FocusPromotionDecisionProjection, FocusPromotionEntityMutation,
    };

    fn entity(id: &str) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: id.into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Use SQLite".into(),
            aliases: vec![],
            scope: KnowledgeScope::Conversation {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
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

    fn focus_update_plan(entity: KnowledgeEntity) -> FocusPromotionDecisionPlan {
        FocusPromotionDecisionPlan {
            decision: FocusPromotionDecisionProjection {
                contract_version: FOCUS_PROMOTION_DECISION_CONTRACT_VERSION.into(),
                decision_id: "decision-vault-recovery-1".into(),
                focus_frame_id: "focus-1".into(),
                conversation_id: "conversation-1".into(),
                candidate_ref: entity.id.clone(),
                action: FocusPromotionDecisionAction::Confirm,
                target_scope: None,
                promoted_entity_id: None,
                source_entity_revision: Some(entity.revision),
                decision_revision: 1,
                memory_version: 1,
                lifecycle_revision: 2,
                decided_at: "2026-08-31T02:00:00Z".into(),
            },
            entity_mutation: FocusPromotionEntityMutation::UpsertSource(Box::new(entity)),
        }
    }

    fn focus_delete_plan(entity_id: &str) -> FocusPromotionDecisionPlan {
        FocusPromotionDecisionPlan {
            decision: FocusPromotionDecisionProjection {
                contract_version: FOCUS_PROMOTION_DECISION_CONTRACT_VERSION.into(),
                decision_id: "decision-vault-delete-1".into(),
                focus_frame_id: "focus-1".into(),
                conversation_id: "conversation-1".into(),
                candidate_ref: entity_id.into(),
                action: FocusPromotionDecisionAction::Delete,
                target_scope: None,
                promoted_entity_id: None,
                source_entity_revision: None,
                decision_revision: 1,
                memory_version: 1,
                lifecycle_revision: 2,
                decided_at: "2026-08-31T02:00:00Z".into(),
            },
            entity_mutation: FocusPromotionEntityMutation::DeleteSource {
                entity_id: entity_id.into(),
                expected_revision: 1,
            },
        }
    }

    fn evidence() -> EvidenceRef {
        EvidenceRef {
            id: "evidence-1".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-1".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:abc".into()),
            excerpt: Some("SQLite keeps the source durable.".into()),
            created_at: "2026-08-30T00:00:00Z".into(),
        }
    }

    fn discussion_log() -> DiscussionLog {
        DiscussionLog {
            contract_version: DISCUSSION_LOG_CONTRACT_VERSION.into(),
            id: "discussion-1".into(),
            scope: DiscussionLogScope::Conversation {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
                focus_frame_id: Some("focus-1".into()),
            },
            title: "SQLite decision".into(),
            body_markdown:
                "## Objective\n\nKeep local state durable.\n\n## Next step\n\nVerify restart."
                    .into(),
            related_entity_ids: vec!["entity-1".into()],
            evidence: vec![evidence()],
            revision: 1,
            created_at: "2026-08-30T00:00:00Z".into(),
            updated_at: "2026-08-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn write_entity_creates_obsidian_readable_stable_projection() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let (path, hash) = vault.write_entity(&entity("entity-1")).expect("projection");
        assert_eq!(path, "entities/entity-1.md");
        assert_eq!(hash.len(), 64);
        assert!(directory.path().join(path).is_file());
    }

    #[test]
    fn write_entity_rejects_path_traversal_ids() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let error = vault
            .write_entity(&entity("../escape"))
            .expect_err("unsafe id");
        assert!(error.to_string().contains("safe for a Vault path"));
    }

    #[test]
    fn entity_projection_writes_evidence_pages_and_relation_wikilinks() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let mut projected = entity("entity-1");
        projected.evidence = vec![ScopedEvidenceRef {
            id: "scoped-evidence-1".into(),
            evidence: evidence(),
            scope: projected.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: projected.generator.clone(),
        }];
        let relation = KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "relation-1".into(),
            kind: KnowledgeRelationKind::Supports,
            source_entity_id: "entity-1".into(),
            target_entity_id: "entity-2".into(),
            scope: projected.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: projected.generator.clone(),
            created_at: projected.created_at.clone(),
            updated_at: projected.updated_at.clone(),
        };

        let (path, _) = vault
            .write_entity_with_relations(&projected, &[relation])
            .expect("write enriched entity");
        let markdown = fs::read_to_string(directory.path().join(path)).expect("entity markdown");
        assert!(markdown.contains("[[entity-2|entity-2]]"));
        assert!(markdown.contains("[[../sources/evidence-1|evidence-1]]"));
        let source = fs::read_to_string(directory.path().join("sources/evidence-1.md"))
            .expect("source page");
        assert!(source.contains("message://message-1/block/0"));
    }

    #[test]
    fn discussion_log_round_trips_editable_body_and_managed_links() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let (path, _) = vault
            .write_discussion_log(&discussion_log())
            .expect("write discussion");
        let markdown = fs::read_to_string(directory.path().join(&path)).expect("discussion");
        assert!(markdown.contains("[[../../entities/entity-1|entity-1]]"));
        assert!(markdown.contains("[[../../sources/evidence-1|evidence-1]]"));

        let edited = markdown
            .replace("# SQLite decision", "# SQLite WAL decision")
            .replace("Verify restart.", "Verify restart and rollback.");
        fs::write(directory.path().join(path), edited).expect("external edit");
        let edit = vault
            .read_discussion_log_edit("discussion-1")
            .expect("read edit");
        assert_eq!(edit.title, "SQLite WAL decision");
        assert!(edit.body_markdown.contains("restart and rollback"));
        assert!(!edit.body_markdown.contains(MANAGED_LINKS_MARKER));
    }

    #[test]
    fn discussion_journal_restores_uncommitted_files_and_finishes_committed_index() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let baseline_log = discussion_log();
        let (baseline_path, baseline_hash) = vault
            .write_discussion_log(&baseline_log)
            .expect("baseline discussion");
        let baseline = DiscussionLogProjection {
            contract_version: DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION.into(),
            log: baseline_log.clone(),
            relative_path: baseline_path,
            content_hash: baseline_hash,
        };
        vault
            .write_discussion_index(std::slice::from_ref(&baseline))
            .expect("baseline index");
        let discussion_path = directory.path().join("logs/discussions/discussion-1.md");
        let source_path = directory.path().join("sources/evidence-1.md");
        let index_path = directory.path().join("indexes/discussions.md");
        let original_discussion = fs::read(&discussion_path).expect("discussion bytes");
        let original_source = fs::read(&source_path).expect("source bytes");
        let original_index = fs::read(&index_path).expect("index bytes");
        let mut changed_log = baseline_log;
        changed_log.revision = 2;
        changed_log.title = "SQLite decision after commit".into();
        changed_log.updated_at = "2026-08-31T01:00:00Z".into();
        changed_log.evidence[0].excerpt = Some("Changed evidence bytes.".into());

        let (_, _, pending) = vault
            .apply_discussion_log(&changed_log)
            .expect("apply uncommitted discussion");
        drop(pending);
        assert_ne!(
            fs::read(&source_path).expect("changed source"),
            original_source
        );
        assert_eq!(
            vault
                .recover_discussion_transactions(std::slice::from_ref(&baseline))
                .expect("recover uncommitted discussion"),
            1
        );
        assert_eq!(
            fs::read(&discussion_path).expect("restored discussion"),
            original_discussion
        );
        assert_eq!(
            fs::read(&source_path).expect("restored source"),
            original_source
        );
        assert_eq!(
            fs::read(&index_path).expect("restored index"),
            original_index
        );

        let (relative_path, content_hash, pending) = vault
            .apply_discussion_log(&changed_log)
            .expect("apply committed discussion");
        drop(pending);
        let committed = DiscussionLogProjection {
            contract_version: DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION.into(),
            log: changed_log,
            relative_path,
            content_hash,
        };
        assert_eq!(
            vault
                .recover_discussion_transactions(std::slice::from_ref(&committed))
                .expect("recover committed discussion"),
            1
        );
        assert!(
            fs::read_to_string(discussion_path)
                .expect("committed discussion")
                .contains("SQLite decision after commit")
        );
        assert!(
            fs::read_to_string(source_path)
                .expect("committed source")
                .contains("Changed evidence bytes.")
        );
        assert!(
            fs::read_to_string(index_path)
                .expect("committed index")
                .contains("SQLite decision after commit")
        );
        assert_eq!(
            fs::read_dir(directory.path().join(".discussion-transactions"))
                .expect("transactions")
                .count(),
            0
        );
    }

    #[test]
    fn startup_recovery_restores_previous_markdown_when_replace_was_interrupted() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let (path, _) = vault
            .write_discussion_log(&discussion_log())
            .expect("write discussion");
        let destination = directory.path().join(path);
        let previous = destination.with_extension("md.previous");
        fs::rename(&destination, &previous).expect("simulate interrupted replace");

        assert_eq!(vault.recover_interrupted_writes().expect("recover"), 1);
        assert!(destination.is_file());
        assert!(!previous.exists());
    }

    #[test]
    fn focus_promotion_journal_rolls_back_uncommitted_and_keeps_committed_projection() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let original = entity("entity-1");
        vault.write_entity(&original).expect("original projection");
        let original_bytes =
            fs::read(directory.path().join("entities/entity-1.md")).expect("original bytes");
        let mut updated = original.clone();
        updated.name = "Use SQLite WAL".into();
        updated.revision = 2;
        updated.updated_at = "2026-08-31T02:00:00Z".into();
        let plan = focus_update_plan(updated.clone());

        let pending = vault
            .apply_focus_promotion_plan(&plan, std::slice::from_ref(&updated), &[])
            .expect("apply pending Vault transaction");
        drop(pending);
        assert_ne!(
            fs::read(directory.path().join("entities/entity-1.md")).expect("updated bytes"),
            original_bytes
        );
        assert_eq!(
            vault
                .recover_focus_promotion_transactions(&HashSet::new())
                .expect("rollback uncommitted transaction"),
            1
        );
        assert_eq!(
            fs::read(directory.path().join("entities/entity-1.md")).expect("restored bytes"),
            original_bytes
        );

        let pending = vault
            .apply_focus_promotion_plan(&plan, std::slice::from_ref(&updated), &[])
            .expect("apply committed Vault transaction");
        drop(pending);
        let committed = HashSet::from([plan.decision.decision_id.clone()]);
        assert_eq!(
            vault
                .recover_focus_promotion_transactions(&committed)
                .expect("keep committed transaction"),
            1
        );
        let committed_markdown = fs::read_to_string(directory.path().join("entities/entity-1.md"))
            .expect("committed markdown");
        assert!(committed_markdown.contains("# Use SQLite WAL"));
        assert_eq!(
            fs::read_dir(directory.path().join(".transactions"))
                .expect("transactions")
                .count(),
            0
        );
    }

    #[test]
    fn focus_delete_journal_restores_neighbors_and_committed_recovery_removes_stale_links() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let deleted = entity("entity-delete");
        let mut survivor = entity("entity-survivor");
        survivor.evidence = vec![ScopedEvidenceRef {
            id: "scoped-delete-survivor".into(),
            evidence: evidence(),
            scope: survivor.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: survivor.generator.clone(),
        }];
        let relation = KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "relation-delete-survivor".into(),
            kind: KnowledgeRelationKind::Supports,
            source_entity_id: survivor.id.clone(),
            target_entity_id: deleted.id.clone(),
            scope: survivor.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: survivor.generator.clone(),
            created_at: survivor.created_at.clone(),
            updated_at: survivor.updated_at.clone(),
        };
        vault
            .write_entity_with_relations(&deleted, std::slice::from_ref(&relation))
            .expect("deleted projection");
        vault
            .write_entity_with_relations(&survivor, std::slice::from_ref(&relation))
            .expect("survivor projection");
        vault
            .write_entity_index(&[deleted.clone(), survivor.clone()])
            .expect("entity index");
        let deleted_path = directory.path().join("entities/entity-delete.md");
        let survivor_path = directory.path().join("entities/entity-survivor.md");
        let source_path = directory.path().join("sources/evidence-1.md");
        let index_path = directory.path().join("indexes/entities.md");
        let original_deleted = fs::read(&deleted_path).expect("deleted bytes");
        let original_survivor = fs::read(&survivor_path).expect("survivor bytes");
        let original_source = fs::read(&source_path).expect("source bytes");
        let original_index = fs::read(&index_path).expect("index bytes");
        let plan = focus_delete_plan(&deleted.id);

        let pending = vault
            .apply_focus_promotion_plan(
                &plan,
                std::slice::from_ref(&survivor),
                std::slice::from_ref(&relation),
            )
            .expect("apply delete");
        assert!(!deleted_path.exists());
        assert!(
            !fs::read_to_string(&survivor_path)
                .expect("rewritten survivor")
                .contains("entity-delete")
        );
        vault
            .rollback_focus_promotion(pending)
            .expect("SQLite conflict rollback");
        assert_eq!(
            fs::read(&deleted_path).expect("restored deleted"),
            original_deleted
        );
        assert_eq!(
            fs::read(&survivor_path).expect("restored survivor"),
            original_survivor
        );
        assert_eq!(
            fs::read(&source_path).expect("restored source"),
            original_source
        );
        assert_eq!(
            fs::read(&index_path).expect("restored index"),
            original_index
        );

        let pending = vault
            .apply_focus_promotion_plan(
                &plan,
                std::slice::from_ref(&survivor),
                std::slice::from_ref(&relation),
            )
            .expect("apply committed delete");
        drop(pending);
        assert_eq!(
            vault
                .recover_focus_promotion_transactions(&HashSet::from([plan
                    .decision
                    .decision_id
                    .clone(),]))
                .expect("recover committed delete"),
            1
        );
        assert!(!deleted_path.exists());
        assert!(
            !fs::read_to_string(survivor_path)
                .expect("committed survivor")
                .contains("entity-delete")
        );
        assert!(source_path.is_file());
        assert!(
            !fs::read_to_string(index_path)
                .expect("committed index")
                .contains("entity-delete")
        );
    }

    #[test]
    fn knowledge_delete_journal_restores_present_entity_and_keeps_committed_absence() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        let deleted = entity("generic-delete");
        let survivor = entity("generic-survivor");
        let relation = KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "generic-delete-relation".into(),
            kind: KnowledgeRelationKind::Supports,
            source_entity_id: survivor.id.clone(),
            target_entity_id: deleted.id.clone(),
            scope: survivor.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: survivor.generator.clone(),
            created_at: survivor.created_at.clone(),
            updated_at: survivor.updated_at.clone(),
        };
        vault
            .write_entity_with_relations(&deleted, std::slice::from_ref(&relation))
            .expect("deleted projection");
        vault
            .write_entity_with_relations(&survivor, std::slice::from_ref(&relation))
            .expect("survivor projection");
        vault
            .write_entity_index(&[deleted.clone(), survivor.clone()])
            .expect("index");
        let deleted_path = directory.path().join("entities/generic-delete.md");
        let survivor_path = directory.path().join("entities/generic-survivor.md");
        let index_path = directory.path().join("indexes/entities.md");
        let original_deleted = fs::read(&deleted_path).expect("deleted bytes");
        let original_survivor = fs::read(&survivor_path).expect("survivor bytes");
        let original_index = fs::read(&index_path).expect("index bytes");

        let pending = vault
            .apply_knowledge_entity_delete(
                &deleted.id,
                &[&survivor],
                std::slice::from_ref(&survivor),
                &[],
            )
            .expect("apply uncommitted delete");
        drop(pending);
        assert_eq!(
            vault
                .recover_knowledge_entity_delete_transactions(
                    &[deleted.clone(), survivor.clone()],
                    std::slice::from_ref(&relation),
                )
                .expect("recover uncommitted delete"),
            1
        );
        assert_eq!(
            fs::read(&deleted_path).expect("restored target"),
            original_deleted
        );
        assert_eq!(
            fs::read(&survivor_path).expect("restored survivor"),
            original_survivor
        );
        assert_eq!(
            fs::read(&index_path).expect("restored index"),
            original_index
        );

        let pending = vault
            .apply_knowledge_entity_delete(
                &deleted.id,
                &[&survivor],
                std::slice::from_ref(&survivor),
                &[],
            )
            .expect("apply committed delete");
        drop(pending);
        assert_eq!(
            vault
                .recover_knowledge_entity_delete_transactions(std::slice::from_ref(&survivor), &[],)
                .expect("recover committed delete"),
            1
        );
        assert!(!deleted_path.exists());
        assert!(
            !fs::read_to_string(survivor_path)
                .expect("committed survivor")
                .contains("[[generic-delete|")
        );
        assert!(
            !fs::read_to_string(index_path)
                .expect("committed index")
                .contains("[[../entities/generic-delete|")
        );
    }

    #[test]
    fn read_entity_edit_returns_changed_heading_and_hash() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        vault.write_entity(&entity("entity-1")).expect("projection");
        let path = directory.path().join("entities/entity-1.md");
        let content = std::fs::read_to_string(&path)
            .expect("read")
            .replace("# Use SQLite", "# Use SQLite WAL");
        std::fs::write(path, content).expect("edit");
        let edit = vault.read_entity_edit("entity-1").expect("import edit");
        assert_eq!(edit.name, "Use SQLite WAL");
    }

    #[test]
    fn read_entity_edit_rejects_stable_id_mismatch() {
        let directory = TempDir::new().expect("temp directory");
        let vault = MarkdownVault::new(directory.path()).expect("vault");
        vault.write_entity(&entity("entity-1")).expect("projection");
        let path = directory.path().join("entities/entity-1.md");
        let content = std::fs::read_to_string(&path)
            .expect("read")
            .replace("\"entity-1\"", "\"entity-2\"");
        std::fs::write(path, content).expect("edit");
        let error = vault.read_entity_edit("entity-1").expect_err("mismatch");
        assert!(error.to_string().contains("stable ID"));
    }
}
