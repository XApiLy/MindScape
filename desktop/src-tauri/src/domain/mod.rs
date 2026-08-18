mod content;
mod context;
mod conversation;
mod credentials;
mod error;

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
