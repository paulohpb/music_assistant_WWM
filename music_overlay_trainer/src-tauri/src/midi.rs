use midly::{Smf, TrackEventKind};
use std::fs;

#[derive(Clone, Debug, serde::Serialize)]
pub struct MidiEvent {
    pub time_ms: u64,
    pub key: String,
    pub duration_ms: u64,
    pub track_index: usize,
}

#[derive(Clone, serde::Serialize)]
pub struct MidiData {
    pub events: Vec<MidiEvent>,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Deserialize)]
pub enum NoteMode {
    Python21,
    Chromatic36,
}

pub fn load_midi(path: &str, mode: NoteMode, transpose: i32) -> Result<MidiData, String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0,
    };

    let mut tempo_changes: Vec<(u64, f64)> = Vec::new();
    let mut max_ticks: u64 = 0;

    // First pass: collect tempo changes
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;
        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            if let TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) = event.kind {
                tempo_changes.push((track_time_ticks, t.as_int() as f64));
            }
        }
        if track_time_ticks > max_ticks {
            max_ticks = track_time_ticks;
        }
    }
    tempo_changes.sort_by_key(|(time, _)| *time);

    let ticks_to_ms = |ticks: u64| -> u64 {
        let mut result_ms = 0.0;
        let mut last_tick = 0u64;
        let mut current_tempo = 500_000.0; // Default 120 BPM (500,000 microseconds per beat)

        for &(change_tick, new_tempo) in &tempo_changes {
            if change_tick >= ticks {
                break;
            }
            let delta_ticks = change_tick - last_tick;
            result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
            last_tick = change_tick;
            current_tempo = new_tempo;
        }
        let delta_ticks = ticks - last_tick;
        result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
        result_ms as u64
    };

    let mut processed_events = Vec::new();

    // Second pass: process notes
    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut track_time_ticks: u64 = 0;
        
        // We need to pair NoteOn with NoteOff. 
        // Simple approach: Store active notes in a map or vec.
        // For rhythm game visualization, we mostly care about "Note On" and duration.
        // Since midly gives a stream, we track active notes.
        // Key: MIDI note number (0-127), Value: Start time (ms)
        let mut active_notes: [Option<u64>; 128] = [None; 128];

        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            let current_ms = ticks_to_ms(track_time_ticks);

            match event.kind {
                TrackEventKind::Midi { channel: _, message } => match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        let note = key.as_int();
                        if vel > 0 {
                            active_notes[note as usize] = Some(current_ms);
                        } else {
                            // Velocity 0 is effectively NoteOff
                            if let Some(start_ms) = active_notes[note as usize] {
                                let key_char = match mode {
                                    NoteMode::Python21 => note_to_key_python(note as i32, transpose),
                                    NoteMode::Chromatic36 => note_to_key_36_closest(note as i32, transpose),
                                };
                                processed_events.push(MidiEvent {
                                    time_ms: start_ms,
                                    key: key_char,
                                    duration_ms: current_ms - start_ms,
                                    track_index: track_idx,
                                });
                                active_notes[note as usize] = None;
                            }
                        }
                    }
                    midly::MidiMessage::NoteOff { key, vel: _ } => {
                        let note = key.as_int();
                        if let Some(start_ms) = active_notes[note as usize] {
                            let key_char = match mode {
                                NoteMode::Python21 => note_to_key_python(note as i32, transpose),
                                NoteMode::Chromatic36 => note_to_key_36_closest(note as i32, transpose),
                            };
                            processed_events.push(MidiEvent {
                                time_ms: start_ms,
                                key: key_char,
                                duration_ms: current_ms - start_ms,
                                track_index: track_idx,
                            });
                            active_notes[note as usize] = None;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    // Sort events by time
    processed_events.sort_by_key(|e| e.time_ms);

    Ok(MidiData {
        events: processed_events,
        duration_ms: ticks_to_ms(max_ticks),
    })
}

// --- MAPPING LOGIC ---

fn note_to_key_python(note: i32, transpose: i32) -> String {
    // 21-key Scale Mapping (Diatonic)
    const PY_INSTRUMENT_NOTES: [i32; 21] = [
        48, 50, 52, 53, 55, 57, 59,  // Low
        60, 62, 64, 65, 67, 69, 71,  // Mid
        72, 74, 76, 77, 79, 81, 83,  // High
    ];
    const PY_KEYS: [&str; 21] = [
        "z", "x", "c", "v", "b", "n", "m",
        "a", "s", "d", "f", "g", "h", "j",
        "q", "w", "e", "r", "t", "y", "u",
    ];

    let mut target = note + transpose;
    let lo = PY_INSTRUMENT_NOTES[0];
    let hi = PY_INSTRUMENT_NOTES[20];

    // Normalize octave
    while target < lo { target += 12; }
    while target > hi { target -= 12; }

    // Find closest note
    let mut best_idx = 0;
    let mut best_dist = (PY_INSTRUMENT_NOTES[0] - target).abs();

    for (i, &inst_note) in PY_INSTRUMENT_NOTES.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }
    PY_KEYS[best_idx].to_string()
}

fn note_to_key_36_closest(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = if target < 60 { 0 } else if target < 72 { 1 } else { 2 };
    
    match semitone {
        0 => match octave { 0 => "z", 1 => "a", _ => "q" }.to_string(),  // C
        1 => match octave { 0 => "shift+z", 1 => "shift+a", _ => "shift+q" }.to_string(), // C#
        2 => match octave { 0 => "x", 1 => "s", _ => "w" }.to_string(),  // D
        3 => match octave { 0 => "ctrl+c", 1 => "ctrl+d", _ => "ctrl+e" }.to_string(),    // D#
        4 => match octave { 0 => "c", 1 => "d", _ => "e" }.to_string(),  // E
        5 => match octave { 0 => "v", 1 => "f", _ => "r" }.to_string(),  // F
        6 => match octave { 0 => "shift+v", 1 => "shift+f", _ => "shift+r" }.to_string(), // F#
        7 => match octave { 0 => "b", 1 => "g", _ => "t" }.to_string(),  // G
        8 => match octave { 0 => "shift+b", 1 => "shift+g", _ => "shift+t" }.to_string(), // G#
        9 => match octave { 0 => "n", 1 => "h", _ => "y" }.to_string(),  // A
        10 => match octave { 0 => "ctrl+m", 1 => "ctrl+j", _ => "ctrl+u" }.to_string(),   // A#
        11 => match octave { 0 => "m", 1 => "j", _ => "u" }.to_string(), // B
        _ => "a".to_string(),
    }
}