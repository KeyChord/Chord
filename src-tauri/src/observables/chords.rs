use crate::chords::Chord;
use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Default, Serialize)]
pub struct ChordRegistryState {
    pub chords: Vec<Chord>,
}

define_observable!(
    pub struct ChordRegistryObservable(ChordRegistryState);
    id: "chord-registry";
);
