//! Tool results and the six failure categories (contracts/mcp-tools.md §Results, FR-013).
//!
//! Every tool call answers a `CallToolResult`: on success the output object as both pretty JSON
//! text and `structuredContent`; on failure a `"<category>: <message>"` text, the same pair under
//! `structuredContent.error`, and `isError: true`. A tool failure is a *result*, not a JSON-RPC
//! error: the agent reads it and can act on the category.

use serde::Serialize;
use serde_json::{json, Value};

/// Why a tool call failed. The set is closed (SC-010): every failure carries exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// The target does not exist, or is outside the caller's project (FR-010).
    NotFound,
    /// The arguments are malformed, or the request makes no sense for its target.
    InvalidInput,
    /// The target's current state does not allow the operation.
    Conflict,
    /// A rule forbids the operation, or the user declined it.
    RefusedByPolicy,
    /// The operation needs the user's answer and none came.
    NeedsConfirmation,
    /// The service could not carry out an allowed, valid operation.
    ServiceError,
}

impl ErrorCategory {
    /// Every category, in contract order.
    pub const ALL: [ErrorCategory; 6] = [
        ErrorCategory::NotFound,
        ErrorCategory::InvalidInput,
        ErrorCategory::Conflict,
        ErrorCategory::RefusedByPolicy,
        ErrorCategory::NeedsConfirmation,
        ErrorCategory::ServiceError,
    ];

    /// The category's wire name, as the agent sees it.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCategory::NotFound => "not_found",
            ErrorCategory::InvalidInput => "invalid_input",
            ErrorCategory::Conflict => "conflict",
            ErrorCategory::RefusedByPolicy => "refused_by_policy",
            ErrorCategory::NeedsConfirmation => "needs_confirmation",
            ErrorCategory::ServiceError => "service_error",
        }
    }
}

/// A failed operation: its category and a message for the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpError {
    pub category: ErrorCategory,
    pub message: String,
}

impl OpError {
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::NotFound, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::InvalidInput, message)
    }

    pub fn service_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::ServiceError, message)
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.category.as_str(), self.message)
    }
}

impl std::error::Error for OpError {}

/// The `CallToolResult` of a successful call whose output is `output`.
pub fn tool_success(output: &Value) -> Value {
    let text = serde_json::to_string_pretty(output).expect("a JSON value always serialises");
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": output,
        "isError": false,
    })
}

/// The `CallToolResult` of a failed call.
pub fn tool_failure(error: &OpError) -> Value {
    json!({
        "content": [{"type": "text", "text": error.to_string()}],
        "structuredContent": {
            "error": {"category": error.category.as_str(), "message": error.message}
        },
        "isError": true,
    })
}
