use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

/// How far Chord has got with a newer release. Nothing is downloaded until someone asks for it.
#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(tag = "status", content = "update", rename_all = "camelCase")]
pub enum AppUpdateState {
    #[default]
    Idle,
    Available {
        version: String,
    },
    Downloading {
        version: String,
        /// Unknown until the download reports its size.
        percent: Option<u32>,
    },
    Installing {
        version: String,
    },
    /// The new build is on disk and takes over the next time Chord starts.
    Ready {
        version: String,
    },
}

impl AppUpdateState {
    /// Whether an update is under way or finished, so a new check has nothing to add.
    pub fn is_applying(&self) -> bool {
        matches!(
            self,
            Self::Downloading { .. } | Self::Installing { .. } | Self::Ready { .. }
        )
    }
}

define_observable!(
    pub struct AppUpdateObservable(AppUpdateState);
    id: "app-update";
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_in_the_shape_the_settings_card_reads() {
        assert_eq!(
            serde_json::to_value(AppUpdateState::Idle).unwrap(),
            json!({ "status": "idle" })
        );
        assert_eq!(
            serde_json::to_value(AppUpdateState::Downloading {
                version: "0.2.4".into(),
                percent: Some(40),
            })
            .unwrap(),
            json!({ "status": "downloading", "update": { "version": "0.2.4", "percent": 40 } })
        );
    }
}
