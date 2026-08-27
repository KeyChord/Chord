use chord_native_protocol::client::HostError;
use std::time::Duration;

/// User-facing failure modes of a native handler invocation. The messages are what ends up in
/// Chord's logs, so each one says what happened *and* what the supervisor did about it.
#[derive(Debug, thiserror::Error)]
pub enum NativeInvocationError {
    #[error("native host is unavailable: {0}")]
    HostUnavailable(String),

    #[error("native handler {handler_id} is disabled: {reason}")]
    HandlerDisabled { handler_id: String, reason: String },

    /// The handler threw a language-level error; the host is unaffected.
    #[error("native handler threw an error: {message}")]
    Thrown { message: String },

    #[error("invalid native handler arguments: {message}")]
    InvalidArguments { message: String },

    #[error("native handler wrapper failed: {message}")]
    WrapperFailure { message: String },

    /// The host process died (trap, segfault, `exit`, …) while running the handler.
    #[error("native host crashed while running handler {handler_id}; a new host was started. {source}")]
    HostCrashed {
        handler_id: String,
        #[source]
        source: HostError,
    },

    #[error("native handler {handler_id} did not finish within {timeout:?}; the native host was killed and restarted")]
    TimedOut {
        handler_id: String,
        timeout: Duration,
    },

    #[error("native handler {handler_id} was aborted; the native host was restarted")]
    Aborted { handler_id: String },

    #[error("native host protocol error: {0}")]
    Protocol(String),
}
