mod content;
mod context;
mod conversation;
mod credentials;
mod error;
mod focus_lifecycle;
mod focus_promotion;
mod focus_query;
mod focused_context;
mod imports;
mod knowledge;
mod knowledge_retrieval;

pub mod contracts;

pub use content::{ContentBlock, blocks_plain_text};
pub use context::{
    ContextCompileInput, ContextConstraint, ContextMessageRef, ContextSnapshot, ContextTurn,
    OmittedContextRef, compile_context,
};
pub use conversation::{
    AppendTurnInput, BranchType, CanvasNodePosition, CanvasViewportState, CompleteTurnInput,
    Conversation, ConversationEdge, ConversationGraph, ConversationNode, ConversationSummary,
    CreateConversationInput, KernelBootstrap, Message, MessageRole, RunState, SCHEMA_VERSION,
    SaveCanvasViewportInput, StartModelRunInput, UpdateNodePositionInput, Workspace, new_id,
    now_timestamp,
};
pub use credentials::{CredentialError, CredentialRef, CredentialResult, SetCredentialInput};
pub use error::{KernelError, KernelResult};
pub use focus_lifecycle::{
    FOCUS_LIFECYCLE_CONTRACT_VERSION, FocusFrameLifecycleAction, FocusFrameLifecycleCommandInput,
    FocusFrameLifecycleError, FocusFrameLifecycleSnapshot, FocusFrameLifecycleStatus,
    close_focus_frame, reopen_focus_frame, transition_focus_frame,
};
pub use focus_promotion::{
    FOCUS_PROMOTION_DECISION_CONTRACT_VERSION, FocusPromotionDecisionAction,
    FocusPromotionDecisionCommandInput, FocusPromotionDecisionError, FocusPromotionDecisionPlan,
    FocusPromotionDecisionProjection, FocusPromotionEntityMutation, FocusPromotionTargetScope,
    plan_focus_promotion_decision,
};
pub use focus_query::{
    FOCUS_QUERY_CONTRACT_VERSION, FocusFrameQueryProjection, validate_focus_frame_query_projection,
};
pub use focused_context::{
    FOCUSED_CONTEXT_CONTRACT_VERSION, FocusedContextCompileInput, FocusedContextSnapshot,
    compile_focused_context, validate_focused_context_snapshot,
};
pub use imports::{ImportBundleValidationError, validate_import_bundle};
pub use knowledge::{
    KNOWLEDGE_CONTEXT_CONTRACT_VERSION, KnowledgeAction, KnowledgeContextCompileError,
    KnowledgeContextCompileInput, KnowledgeContextReference, KnowledgeContextSelection,
    KnowledgeRetrievalCandidate, KnowledgeRetrievalContext, KnowledgeRetrievalDecision,
    KnowledgeTransition, KnowledgeTransitionError, OmittedKnowledgeRef, compile_knowledge_context,
    retrieval_decision, transition_entity,
};
pub use knowledge_retrieval::{
    KNOWLEDGE_RETRIEVAL_PROJECTION_CONTRACT_VERSION, KnowledgeEmbeddingProvenance,
    KnowledgeRetrievalAvailability, KnowledgeRetrievalCandidateProjection,
    KnowledgeRetrievalNotice, KnowledgeRetrievalProjection, KnowledgeRetrievalSource,
};
