//! Shared memory frame buffer for IMFVirtualCamera
//! The Tauri app writes decoded video frames here.
//! The COM media source DLL reads from here.

use anyhow::{anyhow, Result};
use std::ptr;
use tracing::info;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_WRITE,
    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};

const SHARED_MEM_NAME: &str = "VirtualMeetFrameBuffer";
const HEADER_SIZE: usize = 64; // bytes for metadata

/// Header at the start of shared memory
#[repr(C)]
pub struct FrameHeader {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub frame_number: u64,
    pub timestamp_100ns: i64,
    pub data_size: u32,
    pub ready: u32, // 1 = new frame available, 0 = consumed
}

/// Writer side (Tauri app)
pub struct SharedFrameWriter {
    handle: HANDLE,
    ptr: *mut u8,
    total_size: usize,
}

unsafe impl Send for SharedFrameWriter {}
unsafe impl Sync for SharedFrameWriter {}

impl SharedFrameWriter {
    /// Create shared memory for writing frames.
    /// max_frame_size is width * height * 4 (BGRA) + HEADER_SIZE
    pub fn create(width: u32, height: u32) -> Result<Self> {
        let frame_size = (width * height * 4) as usize;
        let total_size = HEADER_SIZE + frame_size;

        let name_wide: Vec<u16> = SHARED_MEM_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let name_pcwstr = PCWSTR(name_wide.as_ptr());

        unsafe {
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                total_size as u32,
                name_pcwstr,
            )
            .map_err(|e| anyhow!("CreateFileMappingW failed: {}", e))?;

            let view = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, total_size);
            let ptr = view.Value as *mut u8;

            if ptr.is_null() {
                let _ = CloseHandle(handle);
                return Err(anyhow!("MapViewOfFile failed: returned null"));
            }

            // Zero out the header region
            ptr::write_bytes(ptr, 0, HEADER_SIZE);

            info!(
                "Shared frame buffer created: {}x{}, {} bytes total",
                width, height, total_size
            );

            Ok(Self {
                handle,
                ptr,
                total_size,
            })
        }
    }

    /// Write a frame (BGRA format, 4 bytes per pixel).
    pub fn write_frame(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        frame_number: u64,
        timestamp_100ns: i64,
    ) -> Result<()> {
        let data_size = data.len();
        if HEADER_SIZE + data_size > self.total_size {
            return Err(anyhow!(
                "Frame too large: {} payload bytes, but only {} available",
                data_size,
                self.total_size - HEADER_SIZE
            ));
        }

        unsafe {
            let header = self.ptr as *mut FrameHeader;
            (*header).width = width;
            (*header).height = height;
            (*header).stride = width * 4;
            (*header).frame_number = frame_number;
            (*header).timestamp_100ns = timestamp_100ns;
            (*header).data_size = data_size as u32;

            // Copy frame data immediately after header
            let data_ptr = self.ptr.add(HEADER_SIZE);
            ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data_size);

            // Signal new frame is ready — store LAST for memory ordering
            std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
            (*header).ready = 1;
        }

        Ok(())
    }
}

impl Drop for SharedFrameWriter {
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
