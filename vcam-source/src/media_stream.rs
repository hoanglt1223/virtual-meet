//! IMFMediaStream implementation
//! Provides video frames to Frame Server when RequestSample() is called.

use std::sync::{Arc, Mutex};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;

use crate::frame_reader::SharedFrameReader;

/// Our virtual camera media stream
#[implement(IMFMediaStream, IMFMediaEventGenerator)]
pub struct VCamMediaStream {
    event_queue: Mutex<Option<IMFMediaEventQueue>>,
    stream_descriptor: Mutex<Option<IMFStreamDescriptor>>,
    frame_reader: Arc<Mutex<Option<SharedFrameReader>>>,
    is_active: Mutex<bool>,
    width: u32,
    height: u32,
    fps_num: u32,
    fps_den: u32,
    frame_count: Mutex<u64>,
}

impl VCamMediaStream {
    pub fn new(width: u32, height: u32, fps: u32) -> Result<Self> {
        Ok(Self {
            event_queue: Mutex::new(None),
            stream_descriptor: Mutex::new(None),
            frame_reader: Arc::new(Mutex::new(None)),
            is_active: Mutex::new(false),
            width,
            height,
            fps_num: fps,
            fps_den: 1,
            frame_count: Mutex::new(0),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        unsafe {
            // Create event queue
            let queue = MFCreateEventQueue()?;
            *self.event_queue.lock().unwrap() = Some(queue);

            // Create media type (BGRA 32-bit)
            let media_type: IMFMediaType = MFCreateMediaType()?;
            media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            media_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            media_type.SetUINT64(
                &MF_MT_FRAME_SIZE,
                ((self.width as u64) << 32) | (self.height as u64),
            )?;
            media_type.SetUINT64(
                &MF_MT_FRAME_RATE,
                ((self.fps_num as u64) << 32) | (self.fps_den as u64),
            )?;
            media_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            media_type.SetUINT32(&MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
            media_type.SetUINT32(
                &MF_MT_DEFAULT_STRIDE,
                (self.width * 4) as u32,
            )?;

            // Create stream descriptor with one media type
            let type_array = [Some(media_type.cast::<IMFMediaType>()?)];
            let sd = MFCreateStreamDescriptor(0, &type_array)?;

            // Select the media type on the handler
            let handler: IMFMediaTypeHandler = sd.GetMediaTypeHandler()?;
            handler.SetCurrentMediaType(&type_array[0].clone().unwrap())?;

            *self.stream_descriptor.lock().unwrap() = Some(sd);
        }

        Ok(())
    }

    pub fn get_stream_descriptor(&self) -> Result<IMFStreamDescriptor> {
        self.stream_descriptor
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Error::new(E_FAIL, "Stream descriptor not initialized".into()))
    }

    pub fn set_frame_reader(&self, reader: SharedFrameReader) {
        *self.frame_reader.lock().unwrap() = Some(reader);
    }

    pub fn start(&self) {
        *self.is_active.lock().unwrap() = true;
    }

    pub fn stop(&self) {
        *self.is_active.lock().unwrap() = false;
    }

    /// Create an IMFSample from shared memory frame data or generate a solid color frame
    fn create_sample(&self) -> Result<IMFSample> {
        let frame_size = (self.width * self.height * 4) as usize;

        unsafe {
            let sample: IMFSample = MFCreateSample()?;
            let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(frame_size as u32)?;

            // Lock buffer to write frame data
            let mut buf_ptr = std::ptr::null_mut();
            let mut max_len = 0u32;
            let mut cur_len = 0u32;
            buffer.Lock(&mut buf_ptr, Some(&mut max_len), Some(&mut cur_len))?;

            // Try to read from shared memory
            let reader_lock = self.frame_reader.lock().unwrap();
            let got_frame = if let Some(ref reader) = *reader_lock {
                if let Some(frame) = reader.read_frame() {
                    let copy_size = frame.data.len().min(frame_size);
                    std::ptr::copy_nonoverlapping(frame.data.as_ptr(), buf_ptr, copy_size);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            drop(reader_lock);

            if !got_frame {
                // Generate a dark blue frame as placeholder
                let pixel: [u8; 4] = [0x40, 0x20, 0x10, 0xFF]; // BGRA: dark blue
                for i in 0..self.width * self.height {
                    let offset = (i * 4) as usize;
                    if offset + 4 <= frame_size {
                        std::ptr::copy_nonoverlapping(
                            pixel.as_ptr(),
                            buf_ptr.add(offset),
                            4,
                        );
                    }
                }
            }

            buffer.Unlock()?;
            buffer.SetCurrentLength(frame_size as u32)?;

            sample.AddBuffer(&buffer)?;

            // Set timestamp
            let mut count = self.frame_count.lock().unwrap();
            let time_per_frame = 10_000_000i64 * self.fps_den as i64 / self.fps_num as i64;
            sample.SetSampleTime(*count as i64 * time_per_frame)?;
            sample.SetSampleDuration(time_per_frame)?;
            *count += 1;

            Ok(sample)
        }
    }
}

// IMFMediaStream implementation
impl IMFMediaStream_Impl for VCamMediaStream {
    fn GetMediaSource(&self) -> Result<IMFMediaSource> {
        Err(Error::new(E_NOTIMPL, "GetMediaSource not implemented".into()))
    }

    fn GetStreamDescriptor(&self) -> Result<IMFStreamDescriptor> {
        self.get_stream_descriptor()
    }

    fn RequestSample(&self, _token: Option<&IUnknown>) -> Result<()> {
        if !*self.is_active.lock().unwrap() {
            return Err(Error::new(MF_E_MEDIA_SOURCE_WRONGSTATE, "Stream not active".into()));
        }

        // Create and deliver sample via event
        let sample = self.create_sample()?;
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe {
                queue.QueueEventParamUnk(
                    MEMediaSample.0 as u32,
                    &GUID::zeroed(),
                    S_OK,
                    &sample,
                )?;
            }
        }

        Ok(())
    }
}

// IMFMediaEventGenerator implementation
impl IMFMediaEventGenerator_Impl for VCamMediaStream {
    fn GetEvent(&self, dwflags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe { queue.GetEvent(dwflags.0) }
        } else {
            Err(Error::new(E_FAIL, "Event queue not initialized".into()))
        }
    }

    fn BeginGetEvent(
        &self,
        pcallback: Option<&IMFAsyncCallback>,
        punkstate: Option<&IUnknown>,
    ) -> Result<()> {
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe { queue.BeginGetEvent(pcallback, punkstate) }
        } else {
            Err(Error::new(E_FAIL, "Event queue not initialized".into()))
        }
    }

    fn EndGetEvent(&self, presult: Option<&IMFAsyncResult>) -> Result<IMFMediaEvent> {
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe { queue.EndGetEvent(presult) }
        } else {
            Err(Error::new(E_FAIL, "Event queue not initialized".into()))
        }
    }

    fn QueueEvent(
        &self,
        met: u32,
        guidextendedtype: *const GUID,
        hrstatus: HRESULT,
        pvvalue: *const PROPVARIANT,
    ) -> Result<()> {
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe { queue.QueueEventParamVar(met, guidextendedtype, hrstatus, pvvalue) }
        } else {
            Err(Error::new(E_FAIL, "Event queue not initialized".into()))
        }
    }
}
