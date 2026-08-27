//! Wire protocol between the Chord main process and the `chord-native-host` helper.
//!
//! Framing: `u32` little-endian payload length followed by a MessagePack payload.
//! The protocol is deliberately platform-neutral; only the transport (Unix socketpair on
//! macOS/Linux, inherited pipe handle on Windows) differs per platform.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Bumped whenever the request/response shapes change incompatibly.
pub const PROTOCOL_VERSION: u16 = 1;

/// The C symbol every native handler library must export.
pub const ENTRYPOINT_V1: &str = "chord_native_run_v1";

/// Upper bound for a single frame. Normal invocation frames are well under 1 KiB.
pub const MAX_FRAME_LEN: u32 = 8 << 20;

/// Size of the host-owned error buffer handed to the native entrypoint.
pub const ERROR_BUFFER_CAPACITY: usize = 64 * 1024;

/// Environment variable naming the inherited file descriptor the host talks over.
pub const HOST_FD_ENV: &str = "CHORD_NATIVE_HOST_FD";

/// Environment variable naming the directory the host may load libraries from.
pub const CACHE_DIR_ENV: &str = "CHORD_NATIVE_CACHE_DIR";

/// Environment variables the host sets around every invocation.
pub mod invocation_env {
    pub const PACKAGE_NAME: &str = "CHORD_PACKAGE_NAME";
    pub const CHORDS_FILE_PATHSLUG: &str = "CHORD_CHORDS_FILE_PATHSLUG";
    pub const HANDLER_ID: &str = "CHORD_HANDLER_ID";
    pub const INVOCATION_ID: &str = "CHORD_INVOCATION_ID";
    pub const FOCUSED_APP_ID: &str = "CHORD_FOCUSED_APP_ID";
}

/// Return codes of `chord_native_run_v1`.
pub mod abi_status {
    pub const SUCCESS: i32 = 0;
    pub const THROWN: i32 = 1;
    pub const INVALID_ARGUMENTS: i32 = 2;
    pub const WRAPPER_FAILURE: i32 = 3;
}

/// Everything the host needs to know about one logical handler. Static handler arguments are
/// resolved once per generation and cached in the host so invocations only carry event arguments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeHandlerRegistration {
    pub handler_id: String,
    pub library_path: PathBuf,
    pub handler_arguments: Vec<String>,
    pub package_name: String,
    pub chords_file_pathslug: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvocationContext {
    pub focused_app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostRequest {
    Hello {
        protocol_version: u16,
    },
    LoadGeneration {
        generation_id: u64,
        handlers: Vec<NativeHandlerRegistration>,
    },
    Invoke {
        generation_id: u64,
        invocation_id: u64,
        handler_id: String,
        event_arguments: Vec<String>,
        repeat: u32,
        context: InvocationContext,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandlerLoadError {
    pub handler_id: String,
    pub library_path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvocationResult {
    Success,
    /// The handler threw a language-level error which the generated wrapper caught.
    Thrown { message: String },
    /// The wrapper rejected the argument vectors.
    InvalidArguments { message: String },
    /// The wrapper failed for another reason (or returned an unknown status code).
    WrapperFailure { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostResponse {
    Hello {
        protocol_version: u16,
        pid: u32,
    },
    GenerationLoaded {
        generation_id: u64,
        library_count: u32,
        handler_count: u32,
    },
    GenerationLoadFailed {
        generation_id: u64,
        errors: Vec<HandlerLoadError>,
    },
    InvocationFinished {
        invocation_id: u64,
        duration_ns: u64,
        result: InvocationResult,
    },
    ProtocolError {
        message: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame length {0} exceeds maximum of {MAX_FRAME_LEN} bytes")]
    TooLarge(u32),
    #[error("connection closed")]
    Closed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encode error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

/// Encodes a message as a single frame.
pub fn encode_frame<T: Serialize>(message: &T) -> Result<Vec<u8>, FrameError> {
    let payload = rmp_serde::to_vec(message)?;
    let len = u32::try_from(payload.len()).map_err(|_| FrameError::TooLarge(u32::MAX))?;
    if len > MAX_FRAME_LEN {
        return Err(FrameError::TooLarge(len));
    }
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Writes one frame. A single `write_all` keeps the length prefix and payload in one syscall
/// for small frames, which matters on the invocation hot path.
pub async fn write_frame<W, T>(writer: &mut W, message: &T) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let frame = encode_frame(message)?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Reads one frame. Rejects oversized length prefixes before allocating.
pub async fn read_frame<R, T>(reader: &mut R) -> Result<T, FrameError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_bytes = [0u8; 4];
    match reader.read_exact(&mut len_bytes).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Err(FrameError::Closed),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len_bytes);
    if len > MAX_FRAME_LEN {
        return Err(FrameError::TooLarge(len));
    }
    let mut payload = vec![0u8; len as usize];
    match reader.read_exact(&mut payload).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Err(FrameError::Closed),
        Err(e) => return Err(e.into()),
    }
    Ok(rmp_serde::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trips_requests() {
        let request = HostRequest::Invoke {
            generation_id: 7,
            invocation_id: 42,
            handler_id: "h".into(),
            event_arguments: vec!["by-letters".into(), "x".into()],
            repeat: 1,
            context: InvocationContext {
                focused_app_id: Some("com.apple.Safari".into()),
            },
        };
        let mut buf = Vec::new();
        write_frame(&mut buf, &request).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let decoded: HostRequest = read_frame(&mut cursor).await.unwrap();
        assert_eq!(decoded, request);
    }

    #[tokio::test]
    async fn rejects_oversized_length_prefix() {
        let mut frame = (MAX_FRAME_LEN + 1).to_le_bytes().to_vec();
        frame.extend_from_slice(&[0; 16]);
        let mut cursor = std::io::Cursor::new(frame);
        let err = read_frame::<_, HostRequest>(&mut cursor).await.unwrap_err();
        assert!(matches!(err, FrameError::TooLarge(_)));
    }

    #[tokio::test]
    async fn truncated_payload_is_closed() {
        let mut frame = 100u32.to_le_bytes().to_vec();
        frame.extend_from_slice(&[0; 10]);
        let mut cursor = std::io::Cursor::new(frame);
        let err = read_frame::<_, HostRequest>(&mut cursor).await.unwrap_err();
        assert!(matches!(err, FrameError::Closed));
    }

    #[tokio::test]
    async fn empty_stream_is_closed() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        let err = read_frame::<_, HostRequest>(&mut cursor).await.unwrap_err();
        assert!(matches!(err, FrameError::Closed));
    }
}

#[cfg(unix)]
pub mod client;
