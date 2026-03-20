//! VirtualMeet Camera Source - COM DLL implementing IMFMediaSource
//! Reads video frames from shared memory and serves them to Windows Frame Server.
//!
//! This DLL is loaded by Windows Frame Server when MFCreateVirtualCamera is called.
//! The Tauri app decodes video and writes frames to "VirtualMeetFrameBuffer" shared memory.
//! This DLL reads those frames when RequestSample() is called.

mod com_server;
mod frame_reader;
mod media_source;
mod media_stream;

pub use com_server::*;
