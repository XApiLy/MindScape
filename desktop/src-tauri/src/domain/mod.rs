mod content;
mod context;
mod conversation;
mod credentials;
mod error;
mod focus_lifecycle;
mod focused_context;
mod imports;
mod knowledge;

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
    FOCUS_LIFECYCLE_CONTRACT_VERSION, FocusFrameLifecycleAction, FocusFrameLifecycleError,
    FocusFrameLifecycleSnapshot, FocusFrameLifecycleStatus, close_focus_frame, reopen_focus_frame,
    transition_focus_frame,
};
pub use focused_context::{
    FOCUSED_CONTEXT_CONTRACT_VERSION, FocusedContextCompileInput, FocusedContextSnapshot,
    compile_focused_context,
};
pub use imports::{ImportBundleValidationError, validate_import_bundle};
pub use knowledge::{
    KNOWLEDGE_CONTEXT_CONTRACT_VERSION, KnowledgeAction, KnowledgeContextCompileError,
    KnowledgeContextCompileInput, KnowledgeContextReference, KnowledgeContextSelection,
    KnowledgeRetrievalCandidate, KnowledgeRetrievalContext, KnowledgeRetrievalDecision,
    KnowledgeTransition, KnowledgeTransitionError, OmittedKnowledgeRef, compile_knowledge_context,
    retrieval_decision, transition_entity,
};
