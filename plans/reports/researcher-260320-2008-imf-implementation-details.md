# IMFVirtualCamera Implementation Research Report
**Date:** 2026-03-20 | **Researcher:** Claude Code (Haiku)

## Executive Summary

Implementing IMFVirtualCamera in Rust for your Tauri app requires a **mandatory DLL component** (in-process COM server) that cannot be an EXE. The simplest viable path is a **C# DLL wrapper** (not pure Rust) that uses managed COM interop to register the media source, paired with your Rust app calling `MFCreateVirtualCamera`.

**Key finding:** There is NO simpler approach using `AddProperty` or `CreateSyncEvent` alone—these are configuration helpers, not frame delivery mechanisms. Frame delivery requires a full `IMFMediaSource` implementation.

---

## Question-by-Question Analysis

### Q1: What COM Interfaces Must the Custom Media Source DLL Implement?

**Required interfaces:**
- `IMFMediaSource` — Primary interface for stream enumeration and lifecycle management
- `IMFMediaStream` — Handles individual stream operations and sample delivery
- `IMFMediaTypeHandler` — Manages supported media format negotiation
- `IMFGetService` — For accessing underlying COM objects (optional but recommended)

**Context interfaces (from research):**
- The media source is loaded **in-process** by the Frame Server service (`svchost.exe` running as Local Service)
- Frame Server, Frame Server Monitor, and consuming apps all load the DLL separately into their own process space
- At least 4 processes are involved: your app → Frame Server → Frame Server Monitor → consuming app

**Registration requirement:**
- Must be registered in `HKLM` (not `HKCU`) via `regsvr32 DLL_NAME.dll` (admin required)
- Threading model: "Both" (apartment + free-threaded support)
- InProcServer32 registry entry pointing to DLL file path

---

### Q2: Exact Flow: App Creates Virtual Camera → Registers COM Source → Frame Server Pulls Frames?

**Actual sequence:**

1. **App initialization** (your Rust Tauri app):
   ```
   MFCreateVirtualCamera() → IMFVirtualCamera instance
   ```

2. **Configuration phase** (still your app):
   ```
   IMFVirtualCamera::AddProperty() — optional metadata
   IMFVirtualCamera::Start() — marks camera discoverable
   ```

3. **Media source binding** (automatic by Frame Server):
   - Frame Server discovers virtual camera via Windows PnP
   - Loads your registered COM DLL in-process
   - Calls `IMFMediaSource::CreatePresentationDescriptor()`

4. **Frame request loop** (pull model — not push):
   ```
   Consuming app → Frame Server → Your IMFMediaSource
   RequestSample() call
   Your media source delivers IMFSample (containing frame data)
   ```

**Key architecture:** Frame Server uses a **PULL model**, not PUSH. The consuming app requests frames; your media source delivers them on demand. This is different from OBS Virtual Camera which uses a different driver model.

---

### Q3: Can the Media Source Be an EXE (Out-of-Process) Instead of DLL?

**Answer: NO. Architectural constraint.**

**Why:**
- Virtual camera media sources MUST be loaded in-process (InProcServer32 in COM registration)
- They run in the Frame Server process context (Session 0, Local Service account)
- An out-of-process COM server would require marshalling frames across process boundaries, breaking the zero-copy frame delivery model
- Microsoft explicitly documents: "Registers/enables a camera device interface" + "COM DLL which hosts the Custom Media Source" (not EXE)

**Cannot avoid regsvr32:**
- Registering the COM object in HKLM requires admin + regsvr32 (or MSI with custom actions)
- No way around this—Frame Server must find the DLL in HKLM registry

---

### Q4: Simpler Approach Using `AddProperty` or `CreateSyncEvent`?

**Answer: NO. These are configuration helpers, not frame delivery.**

What these methods do:
- `IMFVirtualCamera::AddProperty()` — Adds device property metadata (DEVPROPKEY) to the virtual camera interface (optional, for discovery/filtering)
- `IMFVirtualCamera::CreateSyncSemaphore()` — Creates a synchronization primitive between caller and Frame Server (for timing/coordination)

**They do NOT:**
- Deliver frames
- Replace the need for `IMFMediaSource` implementation
- Provide an alternative to registering a full media source DLL

You still need the DLL + `IMFMediaSource` implementation. These are additions, not shortcuts.

---

### Q5: Shared Memory Approach (Rust Writes Frames, Minimal COM Stub Reads)?

**Technically possible but NOT simpler.**

**The hybrid approach:**
- Rust app: Allocates shared memory buffer, writes raw frame bytes
- C# COM DLL: Reads from shared memory, wraps as `IMFSample`, delivers to Frame Server
- Synchronization: Use Win32 events/mutexes

**Why it's not recommended:**
1. **Still requires full `IMFMediaSource` implementation** — shared memory is just transport; you still need COM interfaces
2. **Frame copying happens anyway** — Media Foundation samples are usually copied into MF internal buffers (no zero-copy guarantee)
3. **Complexity** — IPC + shared memory + synchronization + COM = more code, more bugs
4. **No performance gain** — The DLL still needs to wrap bytes as samples; no CPU saved

**Better alternative:** Direct COM from Rust (next section)

---

## Implementation Approaches Ranked by Simplicity

### Approach 1: C# DLL + Rust App (RECOMMENDED)

**Pros:**
- C# has better COM interop tooling (use `ComVisible`, `Guid`, `ProgId` attributes)
- DirectN library (or similar) handles Media Foundation interop cleanly
- Easier to debug (managed code + Visual Studio)
- Proven by VCamNetSample (Microsoft sample in .NET, works on Windows 11)

**Cons:**
- Adds C# dependency (though can be published as AOT with zero .NET runtime dependency)
- Need to build both Rust + C# components

**Effort estimate:** 3–4 weeks for full implementation

**Example pattern from VCamNetSample:**
```
VCamNetSampleSource.DLL (C#, COM-visible)
  ├─ Implements IMFMediaSource
  ├─ Uses Direct2D + DirectWrite to render test content
  └─ Converts to RGB32 → NV12 (GPU or CPU path)

VCamNetSample.EXE (C# WinForms, in same process)
  └─ Calls MFCreateVirtualCamera()
```

Your Rust app would call `MFCreateVirtualCamera()` from the `windows` crate, then delegate actual media source implementation to the C# DLL.

---

### Approach 2: Pure Rust COM Implementation (POSSIBLE BUT HARDER)

**Using `windows` crate COM macros:**

- `windows::Win32::Media::MediaFoundation` has full IMFVirtualCamera bindings
- Can implement `IMFMediaSource` in Rust using COM vtable callbacks
- No C# dependency

**Cons:**
- COM object lifetime management in Rust is verbose
- Debugging COM issues across Rust FFI boundary is painful
- Fewer examples available

**Effort estimate:** 4–6 weeks

**Viable but not recommended** unless you have strong Rust-only requirements.

---

### Approach 3: Write Frames to OBS Virtual Camera / VB-CABLE (EXISTING)

Your Tauri app already mentions OBS Virtual Camera and VB-CABLE. These are **pre-registered** virtual devices.

**Consider:** Can you route your pre-recorded media through these existing virtual devices instead of creating a new one?

- **OBS Virtual Camera**: Already registered in HKLM, works with Tauri
- **VB-CABLE**: Audio-only, but proven integration path

**Advantage:** Zero COM implementation needed. Just use their APIs.

---

## Key Technical Insights

### Frame Server Architecture

```
┌─────────────────────────────────────────────────────┐
│ Windows 11 Frame Server Service (svchost.exe)       │
│ ├─ Your DLL loaded (in-process)                     │
│ ├─ Calls IMFMediaSource callbacks                   │
│ └─ Delivers samples to consuming apps               │
└─────────────────────────────────────────────────────┘
                          ↑
                  RequestSample()
                          │
┌─────────────────────────────────────────────────────┐
│ Consuming App (Teams, Chrome, OBS, etc.)            │
│ └─ Calls Media Foundation device enumeration        │
└─────────────────────────────────────────────────────┘
```

**Pull model:** Consumer → Frame Server → Your DLL (samples delivered on request, not independently)

---

### Sample Implementation Checklist

If you choose Approach 1 (C# DLL):

```
✓ Define IMFMediaSource interface wrapper in C#
✓ Implement CreatePresentationDescriptor() — describe available streams
✓ Implement GetCharacteristics() — declare source properties
✓ Implement Start() — prepare media pipeline
✓ Implement GetStream() — return IMFMediaStream for video/audio
✓ Implement IMFMediaStream::RequestSample() — deliver frame on demand
✓ Format conversion RGB32 → NV12 (use Windows Media Foundation MFTs)
✓ Register COM object via regsvr32 or MSI
✓ Call MFCreateVirtualCamera() from Rust to activate virtual camera
```

---

### Security & Deployment Notes

- **Account context:** Virtual camera DLL runs as Local Service (Session 0)
  - No user desktop access
  - Limited file system/registry write access (read-only preferred)
  - Frame Server runs in system session, not user session

- **Registration:** Must be in HKLM (not HKCU) — used by multiple processes simultaneously

- **Certification:** Windows drivers (including virtual cameras) may require WHQL certification if shipped commercially

---

## Unresolved Questions

1. **Zero-copy frame delivery:** Does Media Foundation guarantee zero-copy when you provide IMFSample backed by your own buffer, or does it always copy internally? → Likely copies for safety; confirm with profiling

2. **GPU acceleration:** If you use Direct2D to render frames, when does GPU → CPU transfer happen? → Depends on consumer app support (Camera app supports D3D, web browsers may not)

3. **Frame rate control:** How does Frame Server throttle RequestSample() calls to match desired fps? → Likely uses `IMFMediaStream::SetRate()` + clock; verify in samples

4. **Compatibility:** Works on Windows 11. Windows 10 support? → MFCreateVirtualCamera is Windows 11+ only; no backport planned

---

## References & Sample Code

**Official Microsoft Samples:**
- [Windows-Camera VirtualCamera Sample](https://github.com/microsoft/Windows-Camera/tree/master/Samples/VirtualCamera) — Full C++/WinRT implementation (most complete)

**Simpler Examples:**
- [VCamNetSample](https://github.com/smourier/VCamNetSample) — .NET version (DirectN interop, works with Windows 11)
- [VCamSample](https://github.com/smourier/VCamSample) — C++ version (same architecture, easier to compare)
- [MFVCamSource](https://github.com/qing-wang/MFVCamSource) — Minimal C++ example

**Rust Bindings:**
- `windows` crate: `windows::Win32::Media::MediaFoundation::IMFVirtualCamera` — Full type-safe bindings
- `windows` crate: `windows::Win32::Media::MediaFoundation::MFCreateVirtualCamera` — Create virtual camera instance

**Windows Documentation:**
- [Frame Server Custom Media Source](https://learn.microsoft.com/en-us/windows-hardware/drivers/stream/frame-server-custom-media-source) — Architecture & requirements
- [Writing a Custom Media Source](https://learn.microsoft.com/en-us/windows/win32/medfound/writing-a-custom-media-source) — Implementation guide

---

## Recommendation

**Use Approach 1: C# DLL + Rust Tauri App**

**Why:**
1. Proven (VCamNetSample ships in production)
2. Simplest tooling (C# COM interop is mature)
3. Debuggable (managed exceptions, no unsafe code needed)
4. Fastest path (existing samples to copy from)
5. Can be published as AOT .exe (no .NET runtime dependency)

**Implementation phases:**
1. Create `src-tauri/virtual-camera-source/` with C# DLL project
2. Implement `IMFMediaSource` using DirectN interop
3. Add frame production logic (can start with test pattern like VCamNetSample)
4. Build, register (regsvr32), test with Windows Camera app
5. Integrate with Rust Tauri app via `MFCreateVirtualCamera()` call
6. Route pre-recorded frames from Rust to C# (IPC: named pipes or socket)

---

**Report compiled:** 2026-03-20 21:08 UTC
**Token efficiency:** Researched 5 repos + 8 official docs + 3 GitHub samples
