//! COM DLL server exports
//! Provides DllGetClassObject, DllCanUnloadNow, DllRegisterServer, DllUnregisterServer
//! These are the standard entry points Windows uses to load and manage COM DLLs.

use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

use crate::media_source::VCamMediaSource;

/// Our virtual camera source CLSID — must match the Tauri app's constant
/// {B4A7E55D-1E7C-4C90-B74A-6D9E3F8A2B10}
pub const CLSID_VCAM_SOURCE: GUID = GUID::from_u128(0xB4A7E55D_1E7C_4C90_B74A_6D9E3F8A2B10);

/// Track outstanding COM object count for DllCanUnloadNow
static OBJECT_COUNT: AtomicU32 = AtomicU32::new(0);
static LOCK_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn increment_object_count() {
    OBJECT_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn decrement_object_count() {
    OBJECT_COUNT.fetch_sub(1, Ordering::Relaxed);
}

/// Class factory for creating VCamMediaSource instances
#[implement(IClassFactory)]
pub struct VCamClassFactory;

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl IClassFactory_Impl for VCamClassFactory {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if punkouter.is_some() {
            return Err(Error::new(CLASS_E_NOAGGREGATION, "Aggregation not supported".into()));
        }

        unsafe {
            if ppvobject.is_null() {
                return Err(Error::new(E_POINTER, "Null output pointer".into()));
            }
            *ppvobject = std::ptr::null_mut();
        }

        // Create and initialize media source
        let source = VCamMediaSource::new()?;
        source.initialize()?;

        increment_object_count();

        // Query the requested interface
        let unknown: IUnknown = source.into();
        unsafe {
            let hr = unknown.query(riid, ppvobject);
            if hr.is_err() {
                decrement_object_count();
            }
            hr.ok()
        }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            LOCK_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            LOCK_COUNT.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

// ============================================================================
// DLL exports — standard COM server entry points
// ============================================================================

/// Called by COM to get a class factory for our CLSID
///
/// # Safety
/// Raw pointers must be valid COM interface pointers allocated by the caller.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    if rclsid.is_null() || riid.is_null() {
        return E_INVALIDARG;
    }

    let clsid = &*rclsid;
    if *clsid != CLSID_VCAM_SOURCE {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory = VCamClassFactory;
    let unknown: IUnknown = factory.into();
    let hr = unknown.query(riid, ppv);
    if hr.is_ok() { S_OK } else { hr }
}

/// Called by COM to check if the DLL can be unloaded
///
/// # Safety
/// Called by COM runtime; no pointer arguments.
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECT_COUNT.load(Ordering::Relaxed) == 0 && LOCK_COUNT.load(Ordering::Relaxed) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// Register COM server in the registry (called by regsvr32)
///
/// # Safety
/// Called by regsvr32; no pointer arguments.
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    match register_server() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

/// Unregister COM server from the registry (called by regsvr32 /u)
///
/// # Safety
/// Called by regsvr32; no pointer arguments.
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    match unregister_server() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

/// Get the path to this DLL
fn get_module_path() -> Result<String> {
    let mut buf = vec![0u16; 260];
    unsafe {
        let len = windows::Win32::System::LibraryLoader::GetModuleFileNameW(
            windows::Win32::Foundation::HMODULE(0),
            &mut buf,
        );
        if len == 0 {
            return Err(Error::from_win32());
        }
        Ok(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Register our COM class in HKLM
fn register_server() -> Result<()> {
    let clsid_str = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        CLSID_VCAM_SOURCE.data1,
        CLSID_VCAM_SOURCE.data2,
        CLSID_VCAM_SOURCE.data3,
        CLSID_VCAM_SOURCE.data4[0],
        CLSID_VCAM_SOURCE.data4[1],
        CLSID_VCAM_SOURCE.data4[2],
        CLSID_VCAM_SOURCE.data4[3],
        CLSID_VCAM_SOURCE.data4[4],
        CLSID_VCAM_SOURCE.data4[5],
        CLSID_VCAM_SOURCE.data4[6],
        CLSID_VCAM_SOURCE.data4[7],
    );

    let key_path = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid_str);
    let inproc_path = format!("{}\\InprocServer32", key_path);

    let dll_path = get_module_path().unwrap_or_else(|_| "vcam_source.dll".to_string());

    let status = std::process::Command::new("reg")
        .args(["add", &format!("HKLM\\{}", key_path), "/ve", "/d", "VirtualMeet Camera Source", "/f"])
        .status();

    if let Ok(s) = status {
        if !s.success() {
            return Err(Error::new(E_FAIL, "Failed to register CLSID key".into()));
        }
    }

    let status = std::process::Command::new("reg")
        .args([
            "add",
            &format!("HKLM\\{}", inproc_path),
            "/ve",
            "/d",
            &dll_path,
            "/f",
        ])
        .status();

    if let Ok(s) = status {
        if !s.success() {
            return Err(Error::new(E_FAIL, "Failed to register InprocServer32".into()));
        }
    }

    let status = std::process::Command::new("reg")
        .args([
            "add",
            &format!("HKLM\\{}", inproc_path),
            "/v",
            "ThreadingModel",
            "/d",
            "Both",
            "/f",
        ])
        .status();

    if let Ok(s) = status {
        if !s.success() {
            return Err(Error::new(E_FAIL, "Failed to set ThreadingModel".into()));
        }
    }

    Ok(())
}

/// Remove our COM class from HKLM
fn unregister_server() -> Result<()> {
    let clsid_str = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        CLSID_VCAM_SOURCE.data1,
        CLSID_VCAM_SOURCE.data2,
        CLSID_VCAM_SOURCE.data3,
        CLSID_VCAM_SOURCE.data4[0],
        CLSID_VCAM_SOURCE.data4[1],
        CLSID_VCAM_SOURCE.data4[2],
        CLSID_VCAM_SOURCE.data4[3],
        CLSID_VCAM_SOURCE.data4[4],
        CLSID_VCAM_SOURCE.data4[5],
        CLSID_VCAM_SOURCE.data4[6],
        CLSID_VCAM_SOURCE.data4[7],
    );

    let key_path = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid_str);

    let _ = std::process::Command::new("reg")
        .args(["delete", &format!("HKLM\\{}", key_path), "/f"])
        .status();

    Ok(())
}
