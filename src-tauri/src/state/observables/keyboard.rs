use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardState {
    pub is_caps_pressed: bool,
    pub is_shift_pressed: bool
}

define_observable!(
    #[derive(Debug)]
    pub struct KeyboardObservable(KeyboardState);
    id: "keyboard";
);
