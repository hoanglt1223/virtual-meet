//! IMFMediaSource implementation
//! The core COM object that Frame Server loads to get video frames.

use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;

use crate::frame_reader::SharedFrameReader;
use crate::media_stream::VCamMediaStream;

/// Default resolution and frame rate
const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;
const DEFAULT_FPS: u32 = 30;

/// Our virtual camera media source.
/// The stream is stored as an IMFMediaStream COM interface (heap-pinned by the windows crate).
/// Use AsImpl to reach VCamMediaStream methods when needed.
#[implement(IMFMediaSource, IMFMediaEventGenerator)]
pub struct VCamMediaSource {
    event_queue: Mutex<Option<IMFMediaEventQueue>>,
    presentation_descriptor: Mutex<Option<IMFPresentationDescriptor>>,
    stream: Mutex<Option<IMFMediaStream>>,
    state: Mutex<SourceState>,
}

#[derive(Clone, Copy, PartialEq)]
enum SourceState {
    Stopped,
    Started,
    Paused,
    Shutdown,
}

impl VCamMediaSource {
    pub fn new() -> Result<Self> {
        Ok(Self {
            event_queue: Mutex::new(None),
            presentation_descriptor: Mutex::new(None),
            stream: Mutex::new(None),
            state: Mutex::new(SourceState::Stopped),
        })
    }

    /// Initialize the source — create stream, presentation descriptor, try to connect shared memory.
    pub fn initialize(&self) -> Result<()> {
        unsafe {
            // Create event queue
            let queue = MFCreateEventQueue()?;
            *self.event_queue.lock().unwrap() = Some(queue);

            // Build stream, set it up, then convert to COM interface (heap-pins it)
            let stream_impl = VCamMediaStream::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_FPS)?;
            stream_impl.initialize()?;

            // Try to connect shared memory reader
            match SharedFrameReader::open() {
                Ok(reader) => stream_impl.set_frame_reader(reader),
                Err(_) => {} // No shared memory yet — placeholder frames will be used
            }

            // Capture stream descriptor before consuming stream_impl
            let sd = stream_impl.get_stream_descriptor()?;

            // Convert to COM interface — this heap-allocates and ref-counts the object
            let stream_iface: IMFMediaStream = stream_impl.into();

            // Build presentation descriptor
            let sd_array = [Some(sd)];
            let pd = MFCreatePresentationDescriptor(Some(&sd_array))?;
            pd.SelectStream(0)?;

            *self.presentation_descriptor.lock().unwrap() = Some(pd);
            *self.stream.lock().unwrap() = Some(stream_iface);
        }

        Ok(())
    }

    /// Helper: call start/stop on the inner VCamMediaStream via AsImpl.
    unsafe fn stream_start(iface: &IMFMediaStream) {
        let inner: &VCamMediaStream = iface.as_impl();
        inner.start();
    }

    unsafe fn stream_stop(iface: &IMFMediaStream) {
        let inner: &VCamMediaStream = iface.as_impl();
        inner.stop();
    }
}

// IMFMediaSource implementation
impl IMFMediaSource_Impl for VCamMediaSource {
    fn GetCharacteristics(&self) -> Result<u32> {
        let state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "Source shut down".into()));
        }
        // MFMEDIASOURCE_IS_LIVE = source produces live data
        Ok(MFMEDIASOURCE_IS_LIVE.0 as u32)
    }

    fn CreatePresentationDescriptor(&self) -> Result<IMFPresentationDescriptor> {
        let state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "Source shut down".into()));
        }

        let pd_lock = self.presentation_descriptor.lock().unwrap();
        if let Some(ref pd) = *pd_lock {
            unsafe { pd.Clone() }
        } else {
            Err(Error::new(E_FAIL, "Presentation descriptor not initialized".into()))
        }
    }

    fn Start(
        &self,
        _ppresentationdescriptor: Option<&IMFPresentationDescriptor>,
        _pguidtimeformat: *const GUID,
        _pvarstartposition: *const PROPVARIANT,
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "Source shut down".into()));
        }

        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_lock {
            unsafe { VCamMediaSource::stream_start(stream) };
        }

        *state = SourceState::Started;

        // Queue MESourceStarted event
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe {
                let value = PROPVARIANT::default();
                queue.QueueEventParamVar(
                    MESourceStarted.0 as u32,
                    &GUID::zeroed(),
                    S_OK,
                    &value,
                )?;
            }
        }

        // Queue MENewStream event with stream interface
        if let Some(ref stream) = *stream_lock {
            if let Some(ref queue) = *queue_lock {
                unsafe {
                    queue.QueueEventParamUnk(
                        MENewStream.0 as u32,
                        &GUID::zeroed(),
                        S_OK,
                        stream,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn Stop(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "Source shut down".into()));
        }

        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_lock {
            unsafe { VCamMediaSource::stream_stop(stream) };
        }

        *state = SourceState::Stopped;

        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe {
                let value = PROPVARIANT::default();
                queue.QueueEventParamVar(
                    MESourceStopped.0 as u32,
                    &GUID::zeroed(),
                    S_OK,
                    &value,
                )?;
            }
        }

        Ok(())
    }

    fn Pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "Source shut down".into()));
        }

        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_lock {
            unsafe { VCamMediaSource::stream_stop(stream) };
        }

        *state = SourceState::Paused;

        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe {
                let value = PROPVARIANT::default();
                queue.QueueEventParamVar(
                    MESourcePaused.0 as u32,
                    &GUID::zeroed(),
                    S_OK,
                    &value,
                )?;
            }
        }

        Ok(())
    }

    fn Shutdown(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        *state = SourceState::Shutdown;

        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_lock {
            unsafe { VCamMediaSource::stream_stop(stream) };
        }

        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref queue) = *queue_lock {
            unsafe {
                queue.Shutdown()?;
            }
        }

        Ok(())
    }
}

// IMFMediaEventGenerator implementation
impl IMFMediaEventGenerator_Impl for VCamMediaSource {
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
