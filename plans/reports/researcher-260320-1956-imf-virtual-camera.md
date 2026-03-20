# IMFVirtualCamera API & Virtual Audio Research Report
**Date:** 2026-03-20 | **Status:** Actionable Implementation Details

---

## 1. IMFVirtualCamera API: Complete Technical Specification

### Function Signature (Windows Rust Crate)
```rust
pub unsafe fn MFCreateVirtualCamera<P3, P4>(
    type_: MFVirtualCameraType,
    lifetime: MFVirtualCameraLifetime,
    access: MFVirtualCameraAccess,
    friendlyname: P3,
    sourceid: P4,
    categories: Option<&[GUID]>,
) -> Result<IMFVirtualCamera>
where
    P3: Param<PCWSTR>,
    P4: Param<PCWSTR>,
```

### C++ Signature (Reference)
```cpp
HRESULT MFCreateVirtualCamera(
  MFVirtualCameraType     type,
  MFVirtualCameraLifetime lifetime,
  MFVirtualCameraAccess   access,
  LPCWSTR                 friendlyName,
  LPCWSTR                 sourceId,
  const GUID              *categories,
  ULONG                   categoryCount,
  IMFVirtualCamera        **virtualCamera
);
```

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `type` | `MFVirtualCameraType` | Currently **only supports** `MFVirtualCameraType_SoftwareCameraSource` |
| `lifetime` | `MFVirtualCameraLifetime` | `Session` = camera removed when disposed; `System` = persists across reboots |
| `access` | `MFVirtualCameraAccess` | `CurrentUser` = user account only; `AllUsers` = requires admin |
| `friendlyName` | PCWSTR | User-readable name. System automatically appends "Windows Virtual Camera" |
| `sourceId` | PCWSTR | **Custom CLSID in "{GUID}" format** — this is your media source COM registration |
| `categories` | Option<&[GUID]> | Optional device categories. Defaults to `KSCATEGORY_VIDEO_CAMERA`, `KSCATEGORY_VIDEO`, `KSCATEGORY_CAPTURE` |

### Return Value
- **Success:** `Result<IMFVirtualCamera>` containing camera instance
- **Failures:**
  - `E_INVALIDARG` — invalid parameters
  - `E_POINTER` — null output pointer
  - `E_ACCESSDENIED` — privacy policy blocks access OR non-admin calling AllUsers

### Key Design Principles
- **Idempotent:** Same parameters = can re-open existing camera
- **Session vs. System:** Session-lifetime cameras vanish on shutdown; System-lifetime persists (requires `Lifetime_System`)
- **AllUsers requires admin:** Non-admin can only create `CurrentUser` cameras
- **Frame Server Architecture:** Virtual camera integrates with Windows Frame Server subsystem — the OS pulls frames from your media source and distributes to apps

---

## 2. Rust `windows` Crate Support

**✅ CONFIRMED:** The `windows` crate (v0.52+) fully exposes this API.

### Module Path
```rust
use windows::Win32::Media::MediaFoundation::{
    MFCreateVirtualCamera,
    IMFVirtualCamera,
    MFVirtualCameraType_SoftwareCameraSource,
    MFVirtualCameraLifetime_Session,
    MFVirtualCameraLifetime_System,
    MFVirtualCameraAccess_CurrentUser,
    MFVirtualCameraAccess_AllUsers,
    KSCATEGORY_VIDEO_CAMERA,
    KSCATEGORY_VIDEO,
    KSCATEGORY_CAPTURE,
    KSCATEGORY_SENSOR_CAMERA,
};
```

### Feature Flags Required
No special feature flags needed — `windows` crate exposes MediaFoundation APIs by default.

### Library Dependencies
- **DLL:** `mfsensorgroup.dll` (Windows 11+)
- **Library (link):** `mfsensorgroup.lib`
- **Header:** `mfvirtualcamera.h` (not needed in Rust but for reference)

---

## 3. Frame Delivery Mechanism

### Architecture
Windows Frame Server **pulls frames** from your custom Media Source (COM object). Your app provides the CLSID of this source in the `sourceId` parameter.

### Frame Delivery Process

1. **Create IMFMediaSource** — Your COM object implements `IMFMediaSource`
2. **Populate IMFSample** — Each frame is wrapped in an `IMFSample`:
   ```rust
   pub unsafe fn MFCreateSample() -> Result<IMFSample>;
   ```
3. **Add media buffer:**
   ```rust
   sample.AddBuffer(&media_buffer)?;
   ```
4. **Set timing metadata:**
   - `sample.SetSampleTime(time_in_100ns_units)?;`
   - `sample.SetSampleDuration(duration)?;`
   - **Timestamps MUST be based on QPC time** (QueryPerformanceCounter)
   - Use `MFGetSystemTime()` wrapper — converts QPC to 100-nanosecond units

5. **Media Foundation picks it up** — Frame Server requests samples via `IMFMediaSource::CreateStreamDescriptor()` and stream reading

### Supported Formats
- **RGB32** — standard uncompressed RGB
- **NV12** — YUV planar format (preferred for performance)
- **Format conversion:** Use Media Foundation's Video Processor MFT for GPU-accelerated conversion

### Implementation Requirements
- Media source **must be a COM object** registered in the registry (HKLM, requires admin for registration)
- Register with `regsvr32.exe` (as admin)
- Path must be **publicly accessible** (not user-restricted directories) — Frame Server loads it from multiple processes
- Implement:
  - `IMFMediaSource` — provides stream descriptors and samples
  - `IMFStreamDescriptor` — describes format (width, height, FPS, etc.)
  - `IMFMediaTypeHandler` — enumerable media type support

### Example Registration
```registry
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{YOUR-GUID}]
@="VirtualCameraSource"
[HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{YOUR-GUID}\InprocServer32]
@="C:\\path\\to\\source.dll"
"ThreadingModel"="Apartment"
```

---

## 4. Minimum Windows Version

**Minimum Windows 11 Build: 22000** (initial Windows 11 release)

**Enhanced Capabilities in Build 22621:**
- New API: `IMFVirtualCamera::GetMediaSource()` — retrieve the camera's IMFMediaSource at runtime
- Allows dynamic adjustment of streaming parameters post-initialization

---

## 5. Code Examples & Sample Projects

### Official Microsoft Samples
- **Windows-Camera repository:** [microsoft/Windows-Camera](https://github.com/microsoft/Windows-Camera) — Samples/VirtualCamera directory with complete C++ implementation
- Demonstrates:
  - Direct2D/DirectWrite graphics rendering
  - GPU rendering via Direct3D when available
  - CPU fallback via WIC bitmaps
  - RGB32 → NV12 format conversion

### Community Examples

**VCamSample (C#/.NET):**
- [smourier/VCamSample](https://github.com/smourier/VCamSample) — Minimal C# example
- [smourier/VCamNetSample](https://github.com/smourier/VCamNetSample) — .NET 9 AOT variant (shows bindings approach)

**Rust Implementation Leads:**
- [qing-wang/MFVCamSource](https://github.com/qing-wang/MFVCamSource) — Custom media source implementation
- Useful for understanding frame delivery patterns

### No Official Rust Example Yet
**Gap:** No official Rust IMFVirtualCamera example exists. Your implementation would be pioneering. Key challenges:
- Must create COM objects (use `windows-implement` macro or manual COM vftable)
- Media Source implementation in Rust requires careful COM interface compliance
- Consider wrapping C++ media source via FFI as intermediate step if tight timeline

---

## 6. Virtual Audio Cable Solutions for Bundling

### Open-Source & Bundleable Options

#### A. Scream (MS-PL License)
**Status:** ✅ Actively maintained, Windows 10/11, signed drivers

**Architecture:**
- Kernel-level WDM audio driver (no userspace component)
- Captures audio at driver level → sends as UDP multicast PCM stream
- **Minimal latency** (kernel-level processing)
- Modular: receiver apps listen on multicast socket

**Bundling Feasibility:** ⚠️ **Moderate Difficulty**
- Requires signed kernel driver installation (certificate needed)
- Must register INF file during app setup
- Requires admin elevation for installation
- No documented "silent install" mechanism in public repo
- Build from source: [github.com/duncanthrax/scream](https://github.com/duncanthrax/scream)

**License:** Microsoft Public License (MS-PL) — compatible with many commercial apps

---

#### B. Virtual Audio Wire (VAW)
**Status:** Open-source but less mature than Scream

- Similar concept to Scream (virtual audio device)
- [github.com/HSpear/virtual-audio-wire](https://github.com/HSpear/virtual-audio-wire)
- Documentation sparse; implementation less proven in production

---

#### C. VB-CABLE (Proprietary but Free)
**Status:** ✅ Gold standard for easy bundling; mature; widely used

**Bundling Feasibility:** ✅ **Excellent**
- Binary download available (signed, no build required)
- Installer supports silent mode
- No open-source code, but licensing permits bundling with commercial apps

**Limitations:**
- Proprietary (not open-source)
- Closed binaries only
- License terms require review for commercial use

**Source:** [vb-audio.com/Cable](https://vb-audio.com/Cable/)

---

### How SoundPad / Voicemod Bundle

Both applications use **signed kernel-mode WDM drivers** packaged in their installers:

**SoundPad Approach:**
- ~7 MB standalone installer
- Includes virtual audio driver + INF registration files
- Single unified MSI/EXE installer (no separate driver setup)

**Voicemod Approach:**
- Bundles driver files in: `C:\Program Files\Voicemod Desktop\driver\`
- Includes `mvvad.inf` (Voicemod Virtual Audio Device descriptor)
- Driver registration happens during app installation

**Common Pattern:**
1. Embed driver binary + INF in installer
2. Elevated installer (or UAC prompt) registers driver
3. INF points to bundled driver file location
4. Register device via `SetupAPI` (Windows setup API) or direct registry writes

---

## 7. Implementation Roadmap for VirtualMeet

### Phase 1: IMFVirtualCamera (Rust)
**Effort:** High | **Duration:** 2-3 weeks

1. Create `windows-implement` COM wrapper for `IMFMediaSource`
2. Implement minimal frame delivery:
   - Pre-recorded video file → NV12 samples
   - Fixed 30 FPS, 1280×720
3. Test with Windows 11 Camera app
4. **Blockers:** COM implementation in Rust; Media Foundation sample creation

---

### Phase 2: Virtual Audio Bundling
**Effort:** Medium | **Duration:** 1-2 weeks

**Recommendation:** Use **VB-CABLE** (if licensing allows) OR **Scream** (if full open-source required)

**Setup Steps (Both Options):**
1. Download/build audio driver binary
2. Embed in installer payload
3. Add setup logic to register driver on install
4. Test registration + device enumeration

---

## Unresolved Questions

1. **Rust COM Implementation:** Does your team have existing COM wrapper patterns? (May influence media source architecture)
2. **Audio driver license requirements:** Can VirtualMeet bundle proprietary VB-CABLE, or must it be fully open-source?
3. **Performance target:** Is sub-100ms latency required for audio? (Affects driver choice)
4. **Installer technology:** NSIS, WiX, or custom? (Affects driver installation approach)
5. **Certificate for driver signing:** Do you have a code-signing certificate for kernel drivers? (Required for custom drivers; Scream/VB-CABLE already signed)

---

## Summary Table: Quick Reference

| Topic | Finding |
|-------|---------|
| **API Available in Rust** | ✅ Yes, `windows` crate v0.52+ |
| **Frame Delivery** | Via COM `IMFMediaSource` + `IMFSample` |
| **Min Windows** | Build 22000 (Windows 11 RTM) |
| **Best Open-Source Audio** | Scream (kernel-mode, MS-PL) |
| **Easiest Audio Bundling** | VB-CABLE (proprietary, mature) |
| **Biggest Rust Gap** | No reference Media Source implementation |
| **COM Requirement** | Yes, media source must be COM object |

---

## Sources
- [Microsoft Learn - MFCreateVirtualCamera](https://learn.microsoft.com/en-us/windows/win32/api/mfvirtualcamera/nf-mfvirtualcamera-mfcreatevirtualcamera)
- [Rust windows crate - MFCreateVirtualCamera docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Media/MediaFoundation/fn.MFCreateVirtualCamera.html)
- [Frame Server Custom Media Source - Microsoft Learn](https://learn.microsoft.com/en-us/windows-hardware/drivers/stream/frame-server-custom-media-source)
- [VCamSample - GitHub](https://github.com/smourier/VCamSample)
- [Scream Virtual Audio Driver - GitHub](https://github.com/duncanthrax/scream)
- [Windows-Camera Samples - Microsoft](https://github.com/microsoft/Windows-Camera/tree/master/Samples/VirtualCamera)
- [VirtualAudioWire - GitHub](https://github.com/HSpear/virtual-audio-wire)
- [VB-CABLE Audio Software](https://vb-audio.com/Cable/)
