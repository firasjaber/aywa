# Windows Hex-like App (Tauri + Rust) — Technical Plan

## Product Goal
A lightweight Windows tray app that:
- Automatically installs and manages the speech model
- Uses a global hotkey for push-to-talk
- Shows an overlay indicator
- Transcribes speech locally
- Copies result to clipboard and pastes into focused input

---

## Architecture Overview

### Layer 1: Tauri Shell
- Tray icon
- Settings window
- Overlay UI
- Model download progress
- Notifications

### Layer 2: Rust Core
- Model management
- Audio capture
- Hotkey handling
- State machine
- Transcription pipeline
- Clipboard & paste
- Logging

---

## Process Model

### Main Process
- UI + orchestration
- Hotkey handling
- Recording control

### Worker Process
- Loads model
- Runs inference
- Communicates via JSON over stdio

---

## State Machine

States:
- FirstRun
- CheckingAssets
- DownloadingAssets
- Ready
- Listening
- Processing
- Completed
- Error

---

## Directory Structure

```
AppData/
  config/
  models/
  runtime/
  cache/
  logs/
  state/
```

---

## Model Manager

Responsibilities:
- Detect installed model
- Verify integrity (checksums)
- Download missing assets
- Extract safely
- Self-heal corrupted installs

### Remote Manifest Example
```json
{
  "models": [
    {
      "id": "parakeet-base-en",
      "version": "1.0.0",
      "files": [...]
    }
  ]
}
```

---

## Audio Pipeline

- Push-to-talk recording
- Mono PCM WAV
- Saved to temp file
- Passed to worker

---

## Hotkey Flow

- Press → start recording
- Release → stop recording
- Show overlay states

---

## Overlay States

- Listening
- Transcribing
- Copied
- Pasted
- Error

---

## Clipboard & Paste

- Always copy result
- Attempt Ctrl+V paste
- Fallback: clipboard only

---

## Backend Abstraction

Rust trait:
```rust
trait SpeechBackend {
    fn load(&mut self);
    fn transcribe(&self);
}
```

---

## GPU Fallback Strategy

- Try GPU
- If fail → fallback to CPU

---

## Settings Example

```json
{
  "hotkey": "Ctrl+Alt+Space",
  "preferred_backend": "auto",
  "paste_after_transcribe": true
}
```

---

## Development Phases

1. UI + hotkey + fake text
2. Audio recording
3. Worker process
4. Model manager
5. Real transcription
6. Polish

---

## First Milestone

Press hotkey → speak → release → text appears in Notepad

---

## Key Principles

- Fully automatic setup
- Always fallback to clipboard
- Keep UI simple
- Use state machine
- Separate inference process

