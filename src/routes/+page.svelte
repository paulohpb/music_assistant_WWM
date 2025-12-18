<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, onDestroy } from "svelte";

  interface MidiEvent {
    time_ms: number;
    key: string;
    duration_ms: number;
    track_index: number;
  }

  interface MidiData {
    events: MidiEvent[];
    duration_ms: number;
  }

  interface MidiSummary {
    track_count: number;
    events: string[];
    last_note_time: number;
  }

  let midiPath = $state("");
  let midiData: MidiData | null = $state(null);
  let isPlaying = $state(false);
  let currentTime = $state(0); // in ms
  let playbackSpeed = $state(1.0);
  let startTime = 0;
  let animationFrameId: number;

  let tracks: number[] = $state([]);
  let selectedTrack = $state(0);
  let isDraggingOver = $state(false);

  let unlistenDrag: () => void;

  onMount(async () => {
    // Test parse_midi_file command - replace with your actual MIDI file path
    const testMidiPath = "/tmp/inputs/test.mid"; // Change this to your MIDI file path
    try {
      const summary = await invoke<MidiSummary>("parse_midi_file", { path: testMidiPath });
      console.log("MIDI Summary:");
      console.log("Track count:", summary.track_count);
      console.log("Last note time (ms):", summary.last_note_time);
      console.table(summary.events);
    } catch (err) {
      console.error("Failed to parse MIDI file:", err);
    }

    // Set up Drag & Drop listener
    unlistenDrag = await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === 'enter') {
            isDraggingOver = true;
        } else if (event.payload.type === 'leave') {
            isDraggingOver = false;
        } else if (event.payload.type === 'drop') {
            isDraggingOver = false;
            if (event.payload.paths.length > 0) {
                midiPath = event.payload.paths[0];
                // Trigger load
                loadMidi(new Event('submit'));
            }
        }
    });
  });

  onDestroy(() => {
    if (unlistenDrag) unlistenDrag();
    if (isPlaying) cancelAnimationFrame(animationFrameId);
  });

  async function loadMidi(e: Event) {
    if (e) e.preventDefault();
    if (!midiPath) return;
    try {
      midiData = await invoke<MidiData>("load_midi_file", { path: midiPath });
      console.log("Loaded MIDI with", midiData.events.length, "events");
      
      // Analyze Tracks
      const uniqueTracks = new Set(midiData.events.map(e => e.track_index));
      tracks = Array.from(uniqueTracks).sort((a,b) => a - b);
      
      // Auto-select Track 1 (often melody) if available, else first available
      if (tracks.includes(1)) {
        selectedTrack = 1;
      } else if (tracks.length > 0) {
        selectedTrack = tracks[0];
      }

      stop();
    } catch (err) {
      console.error("Failed to load MIDI:", err);
      alert("Failed to load MIDI: " + err);
    }
  }

  function play() {
    if (!midiData) return;
    if (isPlaying) return;

    // Start with a delay if from beginning
    if (currentTime >= midiData.duration_ms || currentTime === 0) {
        currentTime = -3000; 
    }

    isPlaying = true;
    startTime = performance.now() - (currentTime / playbackSpeed);
    loop();
  }

  function pause() {
    isPlaying = false;
    cancelAnimationFrame(animationFrameId);
  }

  function stop() {
    pause();
    currentTime = 0;
  }

  function loop() {
    if (!isPlaying) return;

    const now = performance.now();
    currentTime = (now - startTime) * playbackSpeed;

    if (midiData && currentTime > midiData.duration_ms + 2000) {
      stop();
      return;
    }

    animationFrameId = requestAnimationFrame(loop);
  }

  function formatTime(ms: number) {
    const totalSeconds = Math.floor(Math.abs(ms) / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    const sign = ms < 0 ? "-" : "";
    return `${sign}${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  const PREVIEW_WINDOW = 2000; // 2 seconds
  const FALL_HEIGHT = 500; // pixels

  function getVisibleEvents(events: MidiEvent[], current: number, track: number) {
    return events.filter(e => 
      e.track_index === track &&
      e.time_ms >= current - 200 &&
      e.time_ms <= current + PREVIEW_WINDOW
    );
  }

  const KEY_ORDER = [
    "z", "x", "c", "v", "b", "n", "m",
    "a", "s", "d", "f", "g", "h", "j",
    "q", "w", "e", "r", "t", "y", "u"
  ];
  
  function getKeyX(key: string) {
      const k = key.toLowerCase().replace("shift+", "").replace("ctrl+", "");
      const idx = KEY_ORDER.indexOf(k);
      if (idx === -1) return 50; 
      return (idx / KEY_ORDER.length) * 90 + 5; 
  }
</script>

<main class="h-screen bg-gray-900 text-white flex flex-col overflow-hidden relative">
  
  <!-- Drag Overlay -->
  {#if isDraggingOver}
    <div class="absolute inset-0 z-50 bg-teal-500/50 backdrop-blur-sm flex items-center justify-center border-4 border-teal-300 border-dashed m-4 rounded-xl">
      <h2 class="text-4xl font-bold text-white drop-shadow-md">Drop MIDI File Here</h2>
    </div>
  {/if}

  <!-- Header / Controls -->
  <div data-tauri-drag-region class="p-4 bg-gray-800 flex flex-wrap items-center gap-4 border-b border-gray-700 z-10 shadow-md cursor-grab">
    <h1 class="text-xl font-bold text-teal-400 pointer-events-none">Where Winds Trainer</h1>
    
    <form onsubmit={loadMidi} class="flex gap-2 flex-1 min-w-[300px]">
      <input 
        class="flex-1 px-3 py-1 bg-gray-700 border border-gray-600 rounded text-sm focus:border-teal-500 outline-none"
        placeholder="Drag & drop .mid file here"
        bind:value={midiPath} 
      />
      <button type="submit" class="bg-teal-600 hover:bg-teal-500 px-4 py-1 rounded text-sm transition-colors">Load</button>
    </form>

    {#if tracks.length > 0}
      <div class="flex items-center gap-2 bg-gray-700 px-2 py-1 rounded border border-gray-600">
        <span class="text-xs text-gray-400">Track:</span>
        <select bind:value={selectedTrack} class="bg-transparent text-sm outline-none cursor-pointer">
          {#each tracks as t}
            <option value={t}>Track {t}</option>
          {/each}
        </select>
      </div>
    {/if}

    <div class="flex items-center gap-2">
      <button onclick={isPlaying ? pause : play} class="w-10 h-10 rounded-full bg-white text-black font-bold flex items-center justify-center hover:bg-gray-200 shadow-lg active:scale-95 transition-transform">
        {#if isPlaying}⏸{:else}▶{/if}
      </button>
      <button onclick={stop} class="px-3 py-1 bg-red-600/80 hover:bg-red-600 rounded text-sm transition-colors">Stop</button>
    </div>

    <div class="flex items-center gap-2">
        <span class="text-xs text-gray-400">Speed</span>
        <input type="range" min="0.1" max="2.0" step="0.1" bind:value={playbackSpeed} class="w-20 accent-teal-500" />
        <span class="text-xs w-8 text-right">{playbackSpeed.toFixed(1)}x</span>
    </div>

    <div class="ml-auto font-mono text-xl">
      {formatTime(currentTime)}
    </div>
  </div>

  <!-- Visualization Area -->
  <div class="flex-1 relative bg-black/80 overflow-hidden flex flex-col">
    {#if midiData}
      <!-- Lanes Background -->
      <div class="absolute inset-0 flex opacity-20 pointer-events-none">
          {#each KEY_ORDER as k, i}
            <div class="flex-1 border-r border-gray-500/30"></div>
          {/each}
      </div>

      <!-- Hit Line -->
      <div class="absolute inset-x-0 bottom-10 h-1 bg-white/50 z-0 shadow-[0_0_10px_white]"></div>
      
      <!-- Notes -->
      {#each getVisibleEvents(midiData.events, currentTime, selectedTrack) as event (event.time_ms + event.key + event.track_index)}
        {@const timeToHit = event.time_ms - currentTime}
        {@const xPos = getKeyX(event.key)}
        
        <div 
          class="absolute flex items-center justify-center rounded-sm text-white font-bold shadow-lg border border-white/20"
          class:bg-teal-500={timeToHit > 100}
          class:bg-yellow-400={timeToHit <= 100 && timeToHit >= -50}
          class:bg-gray-500={timeToHit < -50}
          style:left="{xPos}%" 
          style:bottom="{ 40 }px"
          style:transform="translateY({ -timeToHit * (FALL_HEIGHT / PREVIEW_WINDOW) }px)"
          style:width="3.5%"
          style:height="30px"
        >
          <span class="text-xs md:text-sm drop-shadow-md">
            {event.key.toUpperCase().replace("SHIFT+", "⇧").replace("CTRL+", "⌃")}
          </span>
        </div>
      {/each}
    {:else}
      <div class="flex items-center justify-center h-full text-gray-500 flex-col gap-2">
        <p class="text-lg">Drag & Drop a MIDI file here</p>
        <p class="text-sm opacity-50">or use the input box above</p>
      </div>
    {/if}
  </div>
</main>