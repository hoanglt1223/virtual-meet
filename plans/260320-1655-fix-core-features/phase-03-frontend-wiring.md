# Phase 3: Dashboard Frontend IPC Wiring

## Context Links
- `src/components/Dashboard.tsx` -- entirely mock state, no `invoke()` calls
- `src/types/index.ts` -- TypeScript types (PlaybackState, MediaFile, etc.)
- Tauri plugins enabled: `shell`, `dialog`, `fs`
- `@tauri-apps/api` for invoke, `@tauri-apps/plugin-dialog` for file picker

## Overview
- **Priority**: P1
- **Status**: pending
- **Description**: Wire Dashboard.tsx to call actual Tauri commands for file selection, audio/video streaming, volume control, and device listing

## Key Insight

Dashboard.tsx has ~250 lines of mock UI with zero backend calls. The `invoke()` calls map directly to the commands we fixed in Phase 1 and 2. Tauri's dialog plugin provides the file picker. The wiring is straightforward.

## Architecture

```
Dashboard.tsx
  |
  |-- [Select Video] --> dialog.open({ filters: video }) --> invoke("start_video_streaming", { path, camera_name })
  |-- [Select Audio] --> dialog.open({ filters: audio }) --> invoke("start_audio_streaming", { path, device_name })
  |-- [Stop Video]   --> invoke("stop_video_streaming")
  |-- [Stop Audio]   --> invoke("stop_audio_streaming")
  |-- [Volume]       --> invoke("set_microphone_volume", { volume })
  |-- [Mute]         --> invoke("toggle_microphone_mute")
  |-- [Device List]  --> invoke("list_output_devices") / invoke("list_virtual_cameras")
```

## Related Code Files

### Files to Modify
- `src/components/Dashboard.tsx` -- Add invoke calls, file picker, device selectors
- `src/types/index.ts` -- Add new request/response types if needed

### Files NOT to Create
- No new components; keep everything in Dashboard.tsx (it's the main interaction point)

## Implementation Steps

### Step 1: Add imports

```tsx
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
```

### Step 2: Add state for devices and streaming status

```tsx
const [outputDevices, setOutputDevices] = useState<string[]>([]);
const [virtualCameras, setVirtualCameras] = useState<string[]>([]);
const [selectedOutputDevice, setSelectedOutputDevice] = useState<string>("");
const [selectedCamera, setSelectedCamera] = useState<string>("");
const [isVideoStreaming, setIsVideoStreaming] = useState(false);
const [isAudioStreaming, setIsAudioStreaming] = useState(false);
const [error, setError] = useState<string | null>(null);
```

### Step 3: Fetch devices on mount

```tsx
useEffect(() => {
  const fetchDevices = async () => {
    try {
      const outputs = await invoke<string[]>("list_output_devices");
      setOutputDevices(outputs);
      if (outputs.length > 0) setSelectedOutputDevice(outputs[0]);

      const cameras = await invoke<string[]>("list_virtual_cameras");
      setVirtualCameras(cameras);
      if (cameras.length > 0) setSelectedCamera(cameras[0]);
    } catch (e) {
      console.error("Failed to fetch devices:", e);
    }
  };
  fetchDevices();
}, []);
```

### Step 4: Implement file selection handlers

```tsx
const handleSelectVideo = async () => {
  try {
    const file = await open({
      multiple: false,
      filters: [{ name: "Video", extensions: ["mp4", "mkv", "avi", "webm", "mov"] }],
    });
    if (file) {
      setSelectedVideo({ id: "selected", name: file.split("\\").pop() || file, path: file, type: "video", createdAt: new Date() });
    }
  } catch (e) {
    setError(`Failed to select video: ${e}`);
  }
};

const handleSelectAudio = async () => {
  try {
    const file = await open({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["mp3", "wav", "ogg", "flac", "aac"] }],
    });
    if (file) {
      setSelectedAudio({ id: "selected", name: file.split("\\").pop() || file, path: file, type: "audio", createdAt: new Date() });
    }
  } catch (e) {
    setError(`Failed to select audio: ${e}`);
  }
};
```

### Step 5: Implement streaming start/stop

```tsx
const handleStartVideoStreaming = async () => {
  if (!selectedVideo) return;
  try {
    setError(null);
    await invoke("start_video_streaming", {
      request: { path: selectedVideo.path, camera_name: selectedCamera || null }
    });
    setIsVideoStreaming(true);
  } catch (e) {
    setError(`Video streaming failed: ${e}`);
  }
};

const handleStopVideoStreaming = async () => {
  try {
    await invoke("stop_video_streaming");
    setIsVideoStreaming(false);
  } catch (e) {
    setError(`Stop video failed: ${e}`);
  }
};

const handleStartAudioStreaming = async () => {
  if (!selectedAudio) return;
  try {
    setError(null);
    await invoke("start_audio_streaming", {
      request: { path: selectedAudio.path, device_name: selectedOutputDevice || null }
    });
    setIsAudioStreaming(true);
    setPlaybackState(prev => ({ ...prev, isPlaying: true }));
  } catch (e) {
    setError(`Audio streaming failed: ${e}`);
  }
};

const handleStopAudioStreaming = async () => {
  try {
    await invoke("stop_audio_streaming");
    setIsAudioStreaming(false);
    setPlaybackState(prev => ({ ...prev, isPlaying: false }));
  } catch (e) {
    setError(`Stop audio failed: ${e}`);
  }
};
```

### Step 6: Wire volume control to backend

```tsx
const handleVolumeChange = async (volume: number) => {
  setPlaybackState(prev => ({ ...prev, volume }));
  try {
    await invoke("set_microphone_volume", { volume });
  } catch (e) {
    console.error("Failed to set volume:", e);
  }
};

const handleMuteToggle = async () => {
  try {
    const newState = await invoke<boolean>("toggle_microphone_mute");
    setIsMuted(newState);
  } catch (e) {
    console.error("Failed to toggle mute:", e);
  }
};
```

### Step 7: Update JSX

Replace the "Select Video Source" button's onClick:
```tsx
<Button variant="outline" size="sm" onClick={handleSelectVideo}>
  <Camera className="h-4 w-4 mr-2" />
  Select Video Source
</Button>
```

Add device selector dropdowns (using native `<select>` or shadcn Select component):
```tsx
{/* Output Device Selector */}
<select value={selectedOutputDevice} onChange={e => setSelectedOutputDevice(e.target.value)}>
  {outputDevices.map(d => <option key={d} value={d}>{d}</option>)}
</select>

{/* Virtual Camera Selector */}
<select value={selectedCamera} onChange={e => setSelectedCamera(e.target.value)}>
  {virtualCameras.map(c => <option key={c} value={c}>{c}</option>)}
</select>
```

Add error display:
```tsx
{error && (
  <Card className="border-red-500">
    <CardContent className="py-2 text-red-500 text-sm">{error}</CardContent>
  </Card>
)}
```

Wire the Quick Action buttons in the sidebar:
- Play/Pause -> `handleStartAudioStreaming` / `handleStopAudioStreaming`
- Stop -> both stop handlers
- "Select Video Source" button -> `handleSelectVideo`

Add an "Audio Source" section similar to video with `handleSelectAudio`.

### Step 8: Remove mock timer

Remove the `useEffect` interval at line 38-54 that fakes playback progress. Real progress will come from polling the backend or from Tauri events (future improvement).

### Step 9: Change `_setSelectedVideo` / `_setSelectedAudio` to active setters

Lines 28-29 use `_setSelectedVideo` and `_setSelectedAudio` (prefixed underscore = unused). Rename to `setSelectedVideo` and `setSelectedAudio` and use them in the file picker handlers.

### Step 10: Run frontend tests

```bash
pnpm type-check
pnpm lint
pnpm test
```

## Todo List

- [ ] Add `invoke` and `open` imports
- [ ] Add device state and fetch on mount
- [ ] Implement `handleSelectVideo` with dialog.open
- [ ] Implement `handleSelectAudio` with dialog.open
- [ ] Implement `handleStartVideoStreaming` / `handleStopVideoStreaming`
- [ ] Implement `handleStartAudioStreaming` / `handleStopAudioStreaming`
- [ ] Wire `handleVolumeChange` to backend invoke
- [ ] Wire `handleMuteToggle` to backend invoke
- [ ] Add device selector dropdowns to UI
- [ ] Add error display card
- [ ] Remove mock timer useEffect
- [ ] Rename `_setSelectedVideo` / `_setSelectedAudio`
- [ ] Run `pnpm type-check && pnpm lint && pnpm test`

## Success Criteria

- User can click "Select Video" and browse for a video file
- User can click "Select Audio" and browse for an audio file
- User can pick output device and virtual camera from dropdowns
- Play/Stop buttons call actual backend commands
- Volume slider adjusts audio playback volume
- Errors from backend display clearly in UI
- No TypeScript errors, lint passes

## Security Considerations

- File paths come from Tauri dialog plugin (system-native, sandboxed)
- No raw user text input goes into file paths
- Device names are selected from enumerated list, not free-text
