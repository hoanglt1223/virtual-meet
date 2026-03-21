//! Quick test: MFCreateVirtualCamera + Start
use windows::core::GUID;
use windows::Win32::Media::KernelStreaming::KSCATEGORY_VIDEO_CAMERA;
use windows::Win32::Media::MediaFoundation::*;

const VCAM_CLSID: GUID = GUID::from_u128(0xB4A7E55D_1E7C_4C90_B74A_6D9E3F8A2B10);

fn main() {
    println!("MFStartup...");
    unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }.expect("MFStartup failed");

    let clsid = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        VCAM_CLSID.data1, VCAM_CLSID.data2, VCAM_CLSID.data3,
        VCAM_CLSID.data4[0], VCAM_CLSID.data4[1],
        VCAM_CLSID.data4[2], VCAM_CLSID.data4[3],
        VCAM_CLSID.data4[4], VCAM_CLSID.data4[5],
        VCAM_CLSID.data4[6], VCAM_CLSID.data4[7],
    );
    println!("CLSID: {}", clsid);

    let vcam = unsafe {
        MFCreateVirtualCamera(
            MFVirtualCameraType_SoftwareCameraSource,
            MFVirtualCameraLifetime_Session,
            MFVirtualCameraAccess_CurrentUser,
            &windows::core::HSTRING::from("VirtualMeet Camera"),
            &windows::core::HSTRING::from(clsid.as_str()),
            None, // No categories — like VCamSample
        )
    };

    match vcam {
        Ok(cam) => {
            println!("MFCreateVirtualCamera OK");

            // Dump existing attributes on the IMFVirtualCamera
            unsafe {
                match cam.GetCount() {
                    Ok(count) => {
                        println!("IMFVirtualCamera has {} attributes", count);
                        for i in 0..count {
                            let mut key = GUID::zeroed();
                            if cam.GetItemByIndex(i, &mut key, None).is_ok() {
                                println!("  Attr[{}]: {:08X}-{:04X}-{:04X}", i,
                                    key.data1, key.data2, key.data3);
                            }
                        }
                    }
                    Err(e) => println!("GetCount failed: {}", e),
                }
            }

            // Set required attributes on IMFVirtualCamera before Start
            unsafe {
                let _ = cam.SetGUID(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
                );
                let _ = cam.SetString(
                    &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                    windows::core::w!("VirtualMeet Camera"),
                );
                // Set media type info
                let _ = cam.SetUINT64(
                    &MF_MT_FRAME_SIZE,
                    ((1280u64) << 32) | (720u64),
                );
            }
            println!("Calling Start()...");
            match unsafe { cam.Start(None) } {
                Ok(()) => {
                    println!("SUCCESS! Camera visible. Sleeping 10s...");
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    let _ = unsafe { cam.Stop() };
                    let _ = unsafe { cam.Remove() };
                    println!("Stopped & removed.");
                }
                Err(e) => {
                    println!("Start FAILED: {} (0x{:08X})", e.message(), e.code().0 as u32);
                    // Try with AllUsers
                    println!("\nRetrying with AllUsers access...");
                    let _ = unsafe { cam.Remove() };
                    let vcam2 = unsafe {
                        MFCreateVirtualCamera(
                            MFVirtualCameraType_SoftwareCameraSource,
                            MFVirtualCameraLifetime_Session,
                            MFVirtualCameraAccess_AllUsers,
                            &windows::core::HSTRING::from("VirtualMeet Camera"),
                            &windows::core::HSTRING::from(clsid.as_str()),
                            None,
                        )
                    };
                    match vcam2 {
                        Ok(cam2) => {
                            match unsafe { cam2.Start(None) } {
                                Ok(()) => {
                                    println!("SUCCESS with AllUsers! Sleeping 10s...");
                                    std::thread::sleep(std::time::Duration::from_secs(10));
                                    let _ = unsafe { cam2.Stop() };
                                    let _ = unsafe { cam2.Remove() };
                                }
                                Err(e2) => println!("AllUsers also failed: {} (0x{:08X})", e2.message(), e2.code().0 as u32),
                            }
                        }
                        Err(e2) => println!("AllUsers create failed: {}", e2),
                    }
                }
            }
        }
        Err(e) => println!("Create FAILED: {} (0x{:08X})", e.message(), e.code().0 as u32),
    }

    if let Ok(log) = std::fs::read_to_string(r"C:\temp\vcam_debug.log") {
        println!("\n=== DLL LOG ===\n{}", log);
    } else {
        println!("\nNo DLL log found");
    }

    unsafe { MFShutdown() }.ok();
}
