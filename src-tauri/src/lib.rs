mod midi;
mod state;

use state::AppState;
use midi::{NoteMode, MidiData, MidiSummary};
use tauri::State;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn load_midi_file(path: String, state: State<AppState>) -> Result<MidiData, String> {
    // Default to Python21 mode for now, or read from state
    let mode = *state.note_mode.lock().unwrap();
    let data = midi::load_midi(&path, mode, 0)?; // 0 transpose for now
    
    *state.midi_data.lock().unwrap() = Some(data.clone());
    
    Ok(data)
}

#[tauri::command(rename_all = "snake_case", alias = "parse_midi_file")]
fn parse_midi(path: String, state: State<AppState>) -> Result<MidiSummary, String> {
    let (summary, midi_data) = midi::parse_midi(&path)?;
    *state.midi_data.lock().unwrap() = Some(midi_data);
    Ok(summary)
}

#[tauri::command]
fn set_mode(mode_str: String, state: State<AppState>) -> Result<(), String> {
    let mut mode = state.note_mode.lock().unwrap();
    *mode = match mode_str.as_str() {
        "36" => NoteMode::Chromatic36,
        _ => NoteMode::Python21,
    };
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![load_midi_file, parse_midi, set_mode])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}