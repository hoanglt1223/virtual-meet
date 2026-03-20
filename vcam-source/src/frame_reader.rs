//! Shared memory frame reader
//! Reads video frames written by the Tauri app from "VirtualMeetFrameBuffer"

use std::ptr;
use windows::Win32::Foundation::*;
use windows::Win32::System::Memory::*;
use windows::core::*;

const SHARED_MEM_NAME: &str = "VirtualMeetFrameBuffer";
const HEADER_SIZE: usize = 64;

/// Frame header matching the writer in shared_frame_buffer.rs
#[repr(C)]
pub struct FrameHeader {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub frame_number: u64,
    pub timestamp_100ns: i64,
    pub data_size: u32,
    pub ready: u32,
}

/// Reads frames from shared memory created by the Tauri app
pub struct SharedFrameReader {
    handle: HANDLE,
    ptr: *const u8,
    total_size: usize,
}

unsafe impl Send for SharedFrameReader {}
unsafe impl Sync for SharedFrameReader {}

impl SharedFrameReader {
    /// Open existing shared memory for reading
    pub fn open() -> Result<Self> {
        let name: Vec<u16> = SHARED_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let handle = OpenFileMappingW(
                FILE_MAP_READ.0,
                false,
                PCWSTR(name.as_ptr()),
            )?;

            // Map a large enough view (we don't know the size yet, use 16MB max)
            let max_size = 16 * 1024 * 1024;
            let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, max_size);
            let ptr = view.Value as *const u8;

            if ptr.is_null() {
                let _ = CloseHandle(handle);
                return Err(Error::from_win32());
            }

            Ok(Self {
                handle,
                ptr,
                total_size: max_size,
            })
        }
    }

    /// Read the current frame header
    pub fn read_header(&self) -> Option<&FrameHeader> {
        if self.ptr.is_null() {
            return None;
        }
        unsafe {
            let header = self.ptr as *const FrameHeader;
            Some(&*header)
        }
    }

    /// Read the current frame data (returns None if no new frame)
    pub fn read_frame(&self) -> Option<FrameData> {
        let header = self.read_header()?;

        // Check if a new frame is ready
        if header.ready == 0 {
            return None;
        }

        let data_size = header.data_size as usize;
        if data_size == 0 || HEADER_SIZE + data_size > self.total_size {
            return None;
        }

        unsafe {
            let data_ptr = self.ptr.add(HEADER_SIZE);
            let mut data = vec![0u8; data_size];
            ptr::copy_nonoverlapping(data_ptr, data.as_mut_ptr(), data_size);

            // Mark as consumed
            let header_mut = self.ptr as *mut FrameHeader;
            std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
            (*header_mut).ready = 0;

            Some(FrameData {
                width: header.width,
                height: header.height,
                stride: header.stride,
                frame_number: header.frame_number,
                timestamp_100ns: header.timestamp_100ns,
                data,
            })
        }
    }

    /// Check if shared memory is connected
    pub fn is_connected(&self) -> bool {
        !self.ptr.is_null()
    }
}

impl Drop for SharedFrameReader {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.ptr as *mut _,
                });
            }
            if !self.handle.is_invalid() {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

/// A decoded frame read from shared memory
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub frame_number: u64,
    pub timestamp_100ns: i64,
    pub data: Vec<u8>,
}
