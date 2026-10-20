//! Bounded, length-prefixed messages on private worker connections.

use crate::app::desktop_app::ObservedApp;
use crate::models::HandlerChordAction;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub struct Hello {
    pub token: String,
    pub channel: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub root: PathBuf,
    pub cwd: PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Request {
    Register {
        module: String,
        raw: serde_json::Value,
        pathslug: String,
        app_id: Option<String>,
        #[serde(with = "toml_args")]
        args: Vec<toml::Value>,
    },
    Invoke {
        #[serde(with = "handler_payload")]
        action: HandlerChordAction,
        app_id: Option<String>,
        count: u32,
    },
    ReleaseSequences(Vec<String>),
    RetainHandlers(Vec<String>),
    Lifecycle {
        app: ObservedApp,
        launch: bool,
    },
    Ping,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub result: Result<serde_json::Value, String>,
}

#[derive(Serialize, Deserialize)]
pub enum HostRequest {
    Mouse(crate::native_input::MouseOptions),
    Press(String),
    Release(String),
    Tap(String),
    GetGlobalHotkey {
        bundle_id: String,
        hotkey_id: String,
    },
    RegisterGlobalHotkey {
        bundle_id: String,
        hotkey_id: String,
    },
    SetAppNeedsRelaunch {
        bundle_id: String,
        needs_relaunch: bool,
    },
    /// Chord resolves app IDs, including those of bare executables that chords files claim.
    AppProcessIds {
        app_id: String,
    },
}

// Ordinary handler arguments avoid a second text encoding/parser. TOML remains
// the lossless fallback for non-finite numbers and dates. Keep this wire-only:
// the frontend's serialization of HandlerChordAction is unchanged.
mod toml_payload {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

    #[derive(Serialize, Deserialize)]
    enum Payload<T> {
        Json(T),
        Toml(String),
    }

    #[derive(Serialize, Deserialize)]
    struct TomlPayload<T> {
        value: T,
    }

    pub fn json_safe(value: &toml::Value) -> bool {
        match value {
            toml::Value::Float(value) => value.is_finite(),
            toml::Value::Datetime(_) => false,
            toml::Value::Array(values) => values.iter().all(json_safe),
            toml::Value::Table(values) => values.iter().all(|(key, value)| {
                // TOML's serde implementation reserves private date map keys.
                !key.starts_with("$__toml_private") && json_safe(value)
            }),
            _ => true,
        }
    }

    pub fn serialize<T: Serialize, S: Serializer>(
        value: &T,
        json_safe: bool,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if json_safe {
            Payload::Json(value).serialize(serializer)
        } else {
            let source =
                toml::to_string(&TomlPayload { value }).map_err(serde::ser::Error::custom)?;
            Payload::<&T>::Toml(source).serialize(serializer)
        }
    }

    pub fn deserialize<'de, T: DeserializeOwned, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<T, D::Error> {
        match Payload::<T>::deserialize(deserializer)? {
            Payload::Json(value) => Ok(value),
            Payload::Toml(source) => {
                let payload: TomlPayload<T> =
                    toml::from_str(&source).map_err(serde::de::Error::custom)?;
                Ok(payload.value)
            }
        }
    }
}

mod toml_args {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Vec<toml::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        super::toml_payload::serialize(
            value,
            value.iter().all(super::toml_payload::json_safe),
            serializer,
        )
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<toml::Value>, D::Error> {
        super::toml_payload::deserialize(deserializer)
    }
}

mod handler_payload {
    use super::HandlerChordAction;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &HandlerChordAction,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        super::toml_payload::serialize(
            value,
            value.event_args.iter().all(super::toml_payload::json_safe),
            serializer,
        )
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HandlerChordAction, D::Error> {
        super::toml_payload::deserialize(deserializer)
    }
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    // Reserve the header in the same buffer so TCP_NODELAY doesn't send the
    // length and payload as two separate writes/packets for every message.
    let mut bytes = vec![0; 4];
    serde_json::to_writer(&mut bytes, value)?;
    let length = bytes.len() - 4;
    ensure!(
        length <= MAX_FRAME_BYTES,
        "package IPC message exceeds 1 MiB"
    );
    bytes[..4].copy_from_slice(&(length as u32).to_be_bytes());
    Ok(bytes)
}

fn frame_length(header: [u8; 4]) -> Result<usize> {
    let length = u32::from_be_bytes(header) as usize;
    ensure!(
        length <= MAX_FRAME_BYTES,
        "package IPC message exceeds 1 MiB"
    );
    Ok(length)
}

pub fn write_frame<T: Serialize>(stream: &mut impl Write, value: &T) -> Result<()> {
    let bytes = encode(value)?;
    stream.write_all(&bytes)?;
    Ok(())
}

pub fn read_frame<T: DeserializeOwned>(stream: &mut impl Read) -> Result<T> {
    let mut header = [0; 4];
    stream.read_exact(&mut header)?;
    let mut bytes = vec![0; frame_length(header)?];
    stream.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn write_async<T: Serialize>(
    stream: &mut tokio::net::TcpStream,
    value: &T,
) -> Result<()> {
    let bytes = encode(value)?;
    stream.write_all(&bytes).await?;
    Ok(())
}

pub async fn read_async<T: DeserializeOwned>(stream: &mut tokio::net::TcpStream) -> Result<T> {
    let mut header = [0; 4];
    stream.read_exact(&mut header).await?;
    let mut bytes = vec![0; frame_length(header)?];
    stream.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_oversized_frames_before_allocating_the_payload() {
        assert!(
            read_frame::<Hello>(&mut ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().as_slice())
                .is_err()
        );
        assert!(encode(&"x".repeat(MAX_FRAME_BYTES)).is_err());
    }

    #[test]
    fn framing_preserves_newlines_and_rejects_truncated_messages() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &Response {
                result: Err("line one\nline two".into()),
            },
        )
        .unwrap();
        assert_eq!(
            read_frame::<Response>(&mut bytes.as_slice())
                .unwrap()
                .result
                .unwrap_err(),
            "line one\nline two"
        );
        bytes.pop();
        assert!(read_frame::<Response>(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn package_arguments_preserve_nonfinite_toml_values() {
        let request = Request::Register {
            module: "fixture.js".into(),
            raw: serde_json::Value::Null,
            pathslug: "fixture.toml".into(),
            app_id: None,
            args: vec![
                toml::Value::Float(f64::NAN),
                toml::Value::Float(f64::INFINITY),
                "literal".into(),
            ],
        };
        let encoded = encode(&request).unwrap();
        let Request::Register { args, .. } = read_frame(&mut encoded.as_slice()).unwrap() else {
            panic!()
        };
        assert!(args[0].as_float().unwrap().is_nan());
        assert_eq!(args[1].as_float(), Some(f64::INFINITY));
        assert_eq!(args[2].as_str(), Some("literal"));
    }

    #[test]
    fn json_fast_path_preserves_argument_types_and_keyboard_context() {
        use crate::models::{ChordKeyboardEvent, Key, KeyEvent};
        use keycode::KeyMappingCode;
        let args = vec![
            toml::Value::Integer(i64::MIN),
            toml::Value::Integer(i64::MAX),
            toml::Value::Float(-0.0),
            toml::Value::Float(1.0),
            true.into(),
            "literal\n雪".into(),
            toml::Value::Table(toml::Table::from_iter([(
                "nested".into(),
                toml::Value::Array(vec![false.into(), "x".into(), 7.into()]),
            )])),
        ];
        let action = HandlerChordAction {
            handler_id: "handler".into(),
            event_args: args.clone(),
            input: "hsr12".into(),
            sequence_id: Some("interval".into()),
            assign_to: Some("state".into()),
            keyboard_event: Some(ChordKeyboardEvent::new(
                &KeyEvent::Press(Key(KeyMappingCode::Backspace)),
                &[Key(KeyMappingCode::ShiftLeft)],
                false,
            )),
            focused_window_id: Some(u32::MAX),
        };
        let request = Request::Invoke {
            action,
            app_id: Some("dev.fixture".into()),
            count: 32,
        };
        let bytes = encode(&request).unwrap();
        let wire: serde_json::Value = read_frame(&mut bytes.as_slice()).unwrap();
        assert!(wire["Invoke"]["action"]["Json"].is_object());
        let Request::Invoke {
            action,
            app_id,
            count,
        } = read_frame(&mut bytes.as_slice()).unwrap()
        else {
            panic!()
        };
        assert_eq!(action.event_args, args);
        assert_eq!(action.input, "hsr12");
        assert!(action.event_args[2].as_float().unwrap().is_sign_negative());
        assert!(matches!(action.event_args[3], toml::Value::Float(_)));
        assert_eq!(action.sequence_id.as_deref(), Some("interval"));
        assert_eq!(action.assign_to.as_deref(), Some("state"));
        assert!(action.keyboard_event.unwrap().shift_key);
        assert_eq!(action.focused_window_id, Some(u32::MAX));
        assert_eq!(app_id.as_deref(), Some("dev.fixture"));
        assert_eq!(count, 32);
    }

    #[test]
    fn lossless_fallback_handles_dates_and_nested_nonfinite_arguments() {
        let date = toml::Value::Datetime("2026-10-12T14:02:17-04:00".parse().unwrap());
        let args = vec![
            date.clone(),
            toml::Value::Table(toml::Table::from_iter([(
                "nested".into(),
                toml::Value::Array(vec![
                    toml::Value::Float(f64::NEG_INFINITY),
                    toml::Value::Float(f64::NAN),
                ]),
            )])),
        ];
        let action = HandlerChordAction {
            handler_id: "handler".into(),
            event_args: args,
            input: "hsr1".into(),
            sequence_id: None,
            assign_to: None,
            keyboard_event: None,
            focused_window_id: Some(u32::MAX),
        };
        let bytes = encode(&Request::Invoke {
            action,
            app_id: None,
            count: 1,
        })
        .unwrap();
        let wire: serde_json::Value = read_frame(&mut bytes.as_slice()).unwrap();
        assert!(wire["Invoke"]["action"]["Toml"].is_string());
        let Request::Invoke { action, .. } = read_frame(&mut bytes.as_slice()).unwrap() else {
            panic!()
        };
        assert_eq!(action.event_args[0], date);
        assert_eq!(action.focused_window_id, Some(u32::MAX));
        let nested = action.event_args[1]["nested"].as_array().unwrap();
        assert_eq!(nested[0].as_float(), Some(f64::NEG_INFINITY));
        assert!(nested[1].as_float().unwrap().is_nan());
    }

    #[test]
    fn json_fast_path_roundtrips_finite_float_bits() {
        // Include subnormals and wide exponents, not only easy decimal values.
        let mut bits = 1_u64;
        for _ in 0..2048 {
            bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                continue;
            }
            let request = Request::Register {
                module: "fixture.js".into(),
                raw: serde_json::Value::Null,
                pathslug: "fixture.toml".into(),
                app_id: None,
                args: vec![toml::Value::Float(value)],
            };
            let bytes = encode(&request).unwrap();
            let Request::Register { args, .. } = read_frame(&mut bytes.as_slice()).unwrap() else {
                panic!()
            };
            assert_eq!(
                args[0].as_float().unwrap().to_bits(),
                bits,
                "float {value:?} changed across IPC"
            );
        }
    }
}
