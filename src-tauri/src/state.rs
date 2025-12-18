use std::sync::Mutex;
use crate::midi::{MidiData, NoteMode};

pub struct AppState {
    pub midi_data: Mutex<Option<MidiData>>,
    pub note_mode: Mutex<NoteMode>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            midi_data: Mutex::new(None),
            note_mode: Mutex::new(NoteMode::Python21),
        }
    }
}
