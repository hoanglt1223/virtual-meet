//! VCam Activate + MediaSource implementation
//! Frame Server expects IMFActivate. On ActivateObject(IMFMediaSource), we create the source.

use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::KernelStreaming::{KSIDENTIFIER, IKsControl, IKsControl_Impl, KSCAMERAPROFILE_Legacy};
use windows::Win32::Media::MediaFoundation::IMFGetService;
use windows::Win32::Media::MediaFoundation::IMFGetService_Impl;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;

use crate::frame_reader::SharedFrameReader;
use crate::media_stream::VCamMediaStream;

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;
const DEFAULT_FPS: u32 = 30;

// ========================================================================
// VCamActivate — the COM object Frame Server instantiates via our CLSID.
// Implements IMFActivate (which extends IMFAttributes).
// ========================================================================

#[implement(IMFActivate)]
pub struct VCamActivate {
    attrs: Mutex<Option<IMFAttributes>>,
    source: Mutex<Option<IMFMediaSource>>,
}

impl VCamActivate {
    pub fn new() -> Result<Self> {
        Ok(Self {
            attrs: Mutex::new(None),
            source: Mutex::new(None),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 10)?;
            if let Some(ref a) = attrs {
                // Required: tell Frame Server we provide associated camera sources
                a.SetUINT32(&MF_VIRTUALCAMERA_PROVIDE_ASSOCIATED_CAMERA_SOURCES, 1)?;
                a.SetGUID(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
                )?;
                a.SetString(
                    &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                    windows::core::w!("VirtualMeet Camera"),
                )?;
            }
            *self.attrs.lock().unwrap() = attrs;
        }
        Ok(())
    }
}

// IMFActivate: ActivateObject creates the media source
impl IMFActivate_Impl for VCamActivate {
    fn ActivateObject(&self, riid: *const GUID, ppv: *mut *mut core::ffi::c_void) -> Result<()> {
        crate::com_server::debug_log("ActivateObject called");

        let mut source_lock = self.source.lock().unwrap();

        // Create source if not already created
        if source_lock.is_none() {
            let src = VCamMediaSource::new()?;
            src.initialize()?;
            let source_ex: IMFMediaSourceEx = src.into();
            let iface: IMFMediaSource = source_ex.cast()?;
            *source_lock = Some(iface);
            crate::com_server::debug_log("MediaSource created in ActivateObject");
        }

        if let Some(ref source) = *source_lock {
            unsafe {
                let unk: IUnknown = source.cast()?;
                let hr = unk.query(riid, ppv);
                crate::com_server::debug_log(&format!("ActivateObject QI result: {:?}", hr));
                hr.ok()
            }
        } else {
            Err(Error::new(E_FAIL, "Source not created".into()))
        }
    }

    fn ShutdownObject(&self) -> Result<()> {
        crate::com_server::debug_log("ShutdownObject called");
        let mut source_lock = self.source.lock().unwrap();
        if let Some(ref source) = *source_lock {
            unsafe {
                let _ = source.Shutdown();
            }
        }
        *source_lock = None;
        Ok(())
    }

    fn DetachObject(&self) -> Result<()> {
        crate::com_server::debug_log("DetachObject called");
        *self.source.lock().unwrap() = None;
        Ok(())
    }
}

// IMFAttributes — delegate to the internal attribute store
impl IMFAttributes_Impl for VCamActivate {
    fn GetItem(&self, guidkey: *const GUID, pvalue: *mut PROPVARIANT) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            unsafe { a.GetItem(guidkey, Some(pvalue)) }
        } else {
            Err(Error::new(E_FAIL, "".into()))
        }
    }
    fn GetItemType(&self, guidkey: *const GUID) -> Result<MF_ATTRIBUTE_TYPE> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetItemType(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn CompareItem(&self, guidkey: *const GUID, value: *const PROPVARIANT) -> Result<BOOL> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.CompareItem(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn Compare(&self, ptheirs: Option<&IMFAttributes>, matchtype: MF_ATTRIBUTES_MATCH_TYPE) -> Result<BOOL> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.Compare(ptheirs, matchtype) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetUINT32(&self, guidkey: *const GUID) -> Result<u32> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            let result = unsafe { a.GetUINT32(guidkey) };
            if result.is_err() {
                let key = unsafe { &*guidkey };
                crate::com_server::debug_log(&format!("GetUINT32 MISS: {:08X}-{:04X}",
                    key.data1, key.data2));
            }
            result
        }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetUINT64(&self, guidkey: *const GUID) -> Result<u64> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetUINT64(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetDouble(&self, guidkey: *const GUID) -> Result<f64> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetDouble(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetGUID(&self, guidkey: *const GUID) -> Result<GUID> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            let result = unsafe { a.GetGUID(guidkey) };
            if result.is_err() {
                let key = unsafe { &*guidkey };
                crate::com_server::debug_log(&format!("GetGUID MISS: {:08X}-{:04X}",
                    key.data1, key.data2));
            }
            result
        }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetStringLength(&self, guidkey: *const GUID) -> Result<u32> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetStringLength(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetString(&self, guidkey: *const GUID, pwszvalue: PWSTR, cchbufsize: u32, pcchlength: *mut u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            unsafe {
                let slice = std::slice::from_raw_parts_mut(pwszvalue.0, cchbufsize as usize);
                a.GetString(guidkey, slice, Some(pcchlength))
            }
        } else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetAllocatedString(&self, guidkey: *const GUID, ppwszvalue: *mut PWSTR, pcchlength: *mut u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetAllocatedString(guidkey, ppwszvalue, pcchlength) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetBlobSize(&self, guidkey: *const GUID) -> Result<u32> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetBlobSize(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetBlob(&self, guidkey: *const GUID, pbuf: *mut u8, cbbufsize: u32, pcbblobsize: *mut u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            unsafe {
                let slice = std::slice::from_raw_parts_mut(pbuf, cbbufsize as usize);
                a.GetBlob(guidkey, slice, Some(pcbblobsize))
            }
        } else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetAllocatedBlob(&self, guidkey: *const GUID, ppbuf: *mut *mut u8, pcbsize: *mut u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetAllocatedBlob(guidkey, ppbuf, pcbsize) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetUnknown(&self, guidkey: *const GUID, riid: *const GUID, ppv: *mut *mut core::ffi::c_void) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            unsafe {
                (Interface::vtable(a).GetUnknown)(Interface::as_raw(a), guidkey, riid, ppv).ok()
            }
        } else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetItem(&self, guidkey: *const GUID, value: *const PROPVARIANT) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetItem(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn DeleteItem(&self, guidkey: *const GUID) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.DeleteItem(guidkey) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn DeleteAllItems(&self) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.DeleteAllItems() } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetUINT32(&self, guidkey: *const GUID, value: u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetUINT32(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetUINT64(&self, guidkey: *const GUID, value: u64) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetUINT64(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetDouble(&self, guidkey: *const GUID, value: f64) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetDouble(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetGUID(&self, guidkey: *const GUID, value: *const GUID) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetGUID(guidkey, value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetString(&self, guidkey: *const GUID, value: &PCWSTR) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetString(guidkey, *value) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetBlob(&self, guidkey: *const GUID, pbuf: *const u8, cbbufsize: u32) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs {
            unsafe {
                let slice = std::slice::from_raw_parts(pbuf, cbbufsize as usize);
                a.SetBlob(guidkey, slice)
            }
        } else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn SetUnknown(&self, guidkey: *const GUID, punknown: Option<&IUnknown>) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.SetUnknown(guidkey, punknown) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn LockStore(&self) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.LockStore() } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn UnlockStore(&self) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.UnlockStore() } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetCount(&self) -> Result<u32> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetCount() } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn GetItemByIndex(&self, index: u32, pguidkey: *mut GUID, pvalue: *mut PROPVARIANT) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.GetItemByIndex(index, pguidkey, Some(pvalue)) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn CopyAllItems(&self, pdest: Option<&IMFAttributes>) -> Result<()> {
        let attrs = self.attrs.lock().unwrap();
        if let Some(ref a) = *attrs { unsafe { a.CopyAllItems(pdest) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
}

// ========================================================================
// VCamMediaSource — the actual media source, created by VCamActivate
// ========================================================================

#[implement(IMFMediaSourceEx, IMFGetService, IKsControl)]
pub struct VCamMediaSource {
    event_queue: Mutex<Option<IMFMediaEventQueue>>,
    presentation_descriptor: Mutex<Option<IMFPresentationDescriptor>>,
    stream: Mutex<Option<IMFMediaStream>>,
    source_attributes: Mutex<Option<IMFAttributes>>,
    state: Mutex<SourceState>,
}

#[derive(Clone, Copy, PartialEq)]
enum SourceState { Stopped, Started, Paused, Shutdown }

impl VCamMediaSource {
    pub fn new() -> Result<Self> {
        Ok(Self {
            event_queue: Mutex::new(None),
            presentation_descriptor: Mutex::new(None),
            stream: Mutex::new(None),
            source_attributes: Mutex::new(None),
            state: Mutex::new(SourceState::Stopped),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        unsafe {
            let queue = MFCreateEventQueue()?;
            *self.event_queue.lock().unwrap() = Some(queue);

            let stream_impl = VCamMediaStream::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_FPS)?;
            stream_impl.initialize()?;

            if let Ok(reader) = SharedFrameReader::open() {
                stream_impl.set_frame_reader(reader);
            }

            let sd = stream_impl.get_stream_descriptor()?;
            let stream_iface: IMFMediaStream = stream_impl.into();

            let sd_array = [Some(sd)];
            let pd = MFCreatePresentationDescriptor(Some(&sd_array))?;
            pd.SelectStream(0)?;

            *self.presentation_descriptor.lock().unwrap() = Some(pd);
            *self.stream.lock().unwrap() = Some(stream_iface);

            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 10)?;
            if let Some(ref a) = attrs {
                // Required sensor profile collection (like MS sample)
                let profile_collection = MFCreateSensorProfileCollection()?;
                let profile = MFCreateSensorProfile(
                    &KSCAMERAPROFILE_Legacy, 0, windows::core::PCWSTR::null(),
                )?;
                profile.AddProfileFilter(0, windows::core::w!("((RES==;FRT<=30,1;SUT==))"))?;
                profile_collection.AddProfile(&profile)?;
                a.SetUnknown(
                    &MF_DEVICEMFT_SENSORPROFILE_COLLECTION,
                    &profile_collection,
                )?;
            }
            *self.source_attributes.lock().unwrap() = attrs;
        }
        Ok(())
    }

    unsafe fn stream_start(iface: &IMFMediaStream) {
        let inner: &VCamMediaStream = iface.as_impl();
        inner.start();
    }
    unsafe fn stream_stop(iface: &IMFMediaStream) {
        let inner: &VCamMediaStream = iface.as_impl();
        inner.stop();
    }
}

impl IMFMediaSourceEx_Impl for VCamMediaSource {
    fn GetSourceAttributes(&self) -> Result<IMFAttributes> {
        crate::com_server::debug_log("GetSourceAttributes called");
        let result = self.source_attributes.lock().unwrap().clone()
            .ok_or_else(|| Error::new(E_FAIL, "".into()));
        crate::com_server::debug_log(&format!("GetSourceAttributes result: {}", result.is_ok()));
        result
    }
    fn GetStreamAttributes(&self, _id: u32) -> Result<IMFAttributes> {
        unsafe {
            let mut a: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut a, 0)?;
            a.ok_or_else(|| Error::new(E_FAIL, "".into()))
        }
    }
    fn SetD3DManager(&self, _mgr: Option<&IUnknown>) -> Result<()> { Ok(()) }
}

impl IMFMediaSource_Impl for VCamMediaSource {
    fn GetCharacteristics(&self) -> Result<u32> {
        crate::com_server::debug_log("GetCharacteristics called");
        if *self.state.lock().unwrap() == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "".into()));
        }
        Ok(MFMEDIASOURCE_IS_LIVE.0 as u32)
    }
    fn CreatePresentationDescriptor(&self) -> Result<IMFPresentationDescriptor> {
        crate::com_server::debug_log("CreatePresentationDescriptor called");
        if *self.state.lock().unwrap() == SourceState::Shutdown {
            return Err(Error::new(MF_E_SHUTDOWN, "".into()));
        }
        let pd = self.presentation_descriptor.lock().unwrap();
        if let Some(ref p) = *pd { unsafe { p.Clone() } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn Start(&self, _pd: Option<&IMFPresentationDescriptor>, _fmt: *const GUID, _pos: *const PROPVARIANT) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown { return Err(Error::new(MF_E_SHUTDOWN, "".into())); }
        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref s) = *stream_lock { unsafe { VCamMediaSource::stream_start(s) }; }
        *state = SourceState::Started;
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref q) = *queue_lock {
            unsafe {
                let v = PROPVARIANT::default();
                q.QueueEventParamVar(MESourceStarted.0 as u32, &GUID::zeroed(), S_OK, &v)?;
            }
        }
        if let Some(ref s) = *stream_lock {
            if let Some(ref q) = *queue_lock {
                unsafe { q.QueueEventParamUnk(MENewStream.0 as u32, &GUID::zeroed(), S_OK, s)?; }
            }
        }
        Ok(())
    }
    fn Stop(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown { return Err(Error::new(MF_E_SHUTDOWN, "".into())); }
        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref s) = *stream_lock { unsafe { VCamMediaSource::stream_stop(s) }; }
        *state = SourceState::Stopped;
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref q) = *queue_lock {
            unsafe { let v = PROPVARIANT::default(); q.QueueEventParamVar(MESourceStopped.0 as u32, &GUID::zeroed(), S_OK, &v)?; }
        }
        Ok(())
    }
    fn Pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if *state == SourceState::Shutdown { return Err(Error::new(MF_E_SHUTDOWN, "".into())); }
        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref s) = *stream_lock { unsafe { VCamMediaSource::stream_stop(s) }; }
        *state = SourceState::Paused;
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref q) = *queue_lock {
            unsafe { let v = PROPVARIANT::default(); q.QueueEventParamVar(MESourcePaused.0 as u32, &GUID::zeroed(), S_OK, &v)?; }
        }
        Ok(())
    }
    fn Shutdown(&self) -> Result<()> {
        *self.state.lock().unwrap() = SourceState::Shutdown;
        let stream_lock = self.stream.lock().unwrap();
        if let Some(ref s) = *stream_lock { unsafe { VCamMediaSource::stream_stop(s) }; }
        let queue_lock = self.event_queue.lock().unwrap();
        if let Some(ref q) = *queue_lock { unsafe { q.Shutdown()?; } }
        Ok(())
    }
}

impl IMFMediaEventGenerator_Impl for VCamMediaSource {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        let q = self.event_queue.lock().unwrap();
        if let Some(ref q) = *q { unsafe { q.GetEvent(flags.0) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn BeginGetEvent(&self, cb: Option<&IMFAsyncCallback>, state: Option<&IUnknown>) -> Result<()> {
        let q = self.event_queue.lock().unwrap();
        if let Some(ref q) = *q { unsafe { q.BeginGetEvent(cb, state) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn EndGetEvent(&self, result: Option<&IMFAsyncResult>) -> Result<IMFMediaEvent> {
        let q = self.event_queue.lock().unwrap();
        if let Some(ref q) = *q { unsafe { q.EndGetEvent(result) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
    fn QueueEvent(&self, met: u32, ext: *const GUID, hr: HRESULT, val: *const PROPVARIANT) -> Result<()> {
        let q = self.event_queue.lock().unwrap();
        if let Some(ref q) = *q { unsafe { q.QueueEventParamVar(met, ext, hr, val) } }
        else { Err(Error::new(E_FAIL, "".into())) }
    }
}

impl IMFGetService_Impl for VCamMediaSource {
    fn GetService(&self, _guidservice: *const GUID, _riid: *const GUID, _ppvobject: *mut *mut core::ffi::c_void) -> Result<()> {
        Err(Error::new(MF_E_UNSUPPORTED_SERVICE, "".into()))
    }
}

impl IKsControl_Impl for VCamMediaSource {
    fn KsProperty(&self, _p: *const KSIDENTIFIER, _pl: u32, _d: *mut core::ffi::c_void, _dl: u32, _br: *mut u32) -> Result<()> {
        Err(Error::new(HRESULT(-2147024809i32), "".into()))
    }
    fn KsMethod(&self, _m: *const KSIDENTIFIER, _ml: u32, _d: *mut core::ffi::c_void, _dl: u32, _br: *mut u32) -> Result<()> {
        Err(Error::new(HRESULT(-2147024809i32), "".into()))
    }
    fn KsEvent(&self, _e: *const KSIDENTIFIER, _el: u32, _d: *mut core::ffi::c_void, _dl: u32, _br: *mut u32) -> Result<()> {
        Err(Error::new(HRESULT(-2147024809i32), "".into()))
    }
}
