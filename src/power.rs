use std::ptr;
use windows_sys::Win32::System::Power::{
    PowerEnumerate, PowerGetActiveScheme, PowerReadACValueIndex, PowerReadFriendlyName,
    PowerSetActiveScheme, PowerWriteACValueIndex, PowerProfileSubGroup, PowerSchemePersonality,
    ACCESS_SCHEME, GUID_POWER_SCHEME_PAGE, SUB_GUID_POWERSCHEME,
};
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, ERROR_MORE_DATA};

#[derive(Debug, Clone)]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub is_active: bool,
}

pub fn get_power_plans() -> Vec<PowerPlan> {
    let mut plans = Vec::new();
    let mut active_guid: Option<String> = None;

    unsafe {
        let mut active_ptr: *mut u8 = ptr::null_mut();
        if PowerGetActiveScheme(0, &mut active_ptr) == ERROR_SUCCESS {
            if !active_ptr.is_null() {
                active_guid = std::ffi::CStr::from_ptr(active_ptr as *const i8)
                    .to_string_lossy()
                    .into_owned()
                    .into();
            }
            if !active_ptr.is_null() {
                windows_sys::Win32::System::Memory::LocalFree(active_ptr as *mut _);
            }
        }

        let mut scheme_index: u32 = 0;
        loop {
            let mut buffer = [0u16; 512];
            let mut buffer_size = buffer.len() as u32;
            let hr = PowerEnumerate(
                0,
                &GUID_POWER_SCHEME_PAGE,
                &SUB_GUID_POWERSCHEME,
                ACCESS_SCHEME,
                &mut scheme_index,
                buffer.as_mut_ptr(),
                &mut buffer_size,
            );
            if hr == ERROR_NO_MORE_ITEMS {
                break;
            }
            if hr != ERROR_SUCCESS && hr != ERROR_MORE_DATA {
                break;
            }

            let scheme_guid = String::from_utf16_lossy(&buffer[..buffer_size as usize])
                .trim_end_matches('\0')
                .to_string();

            let mut friendly: [u16; 260] = [0; 260];
            let mut friendly_size = friendly.len() as u32;
            let mut name = String::new();
            if PowerReadFriendlyName(
                0,
                &GUID_POWER_SCHEME_PAGE,
                &SUB_GUID_POWERSCHEME,
                std::ptr::null(),
                friendly.as_mut_ptr(),
                &mut friendly_size,
            ) == ERROR_SUCCESS {
                name = String::from_utf16_lossy(&friendly[..friendly_size as usize])
                    .trim_end_matches('\0')
                    .to_string();
            }
            if name.is_empty() {
                name = scheme_guid.clone();
            }

            let is_active = active_guid.as_deref() == Some(scheme_guid.as_str());
            plans.push(PowerPlan { guid: scheme_guid, name, is_active });
        }
    }

    if plans.is_empty() {
        plans = vec![
            ("381b4222-f694-41f0-9685-ff5bb260df2e", "Balanced"),
            ("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c", "High performance"),
            ("a1841308-3541-4fab-bc81-f71556f20b4a", "Power saver"),
        ]
        .into_iter()
        .map(|(guid, name)| {
            let is_active = active_guid.as_deref() == Some(guid);
            PowerPlan { guid: guid.to_string(), name: name.to_string(), is_active }
        })
        .collect();
    }

    plans
}

pub fn get_ac_display_brightness(guid: &str) -> Option<u32> {
    unsafe {
        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let guid_wide: Vec<u16> = guid.encode_utf16().chain([0]).collect();
        if PowerReadACValueIndex(
            0,
            &GUID_POWER_SCHEME_PAGE,
            &SUB_GUID_POWERSCHEME,
            &PowerProfileSubGroup { data: [0; 16] },
            &PowerSchemePersonality { data: [0; 16] },
            &mut data,
            &mut size,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        Some(data)
    }
}

pub fn set_ac_display_brightness(guid: &str, brightness: u32) -> Result<(), String> {
    unsafe {
        let guid_wide: Vec<u16> = guid.encode_utf16().chain([0]).collect();
        let mut brightness = brightness;
        let mut size = std::mem::size_of::<u32>() as u32;
        if PowerWriteACValueIndex(
            0,
            &GUID_POWER_SCHEME_PAGE,
            &SUB_GUID_POWERSCHEME,
            &PowerProfileSubGroup { data: [0; 16] },
            &PowerSchemePersonality { data: [0; 16] },
            &mut brightness,
            &mut size,
        ) != ERROR_SUCCESS
        {
            return Err("PowerWriteACValueIndex failed".to_string());
        }
        Ok(())
    }
}

pub fn set_active_power_plan(guid: &str) -> Result<(), String> {
    unsafe {
        let guid_wide: Vec<u16> = guid.encode_utf16().chain([0]).collect();
        let res = PowerSetActiveScheme(0, guid_wide.as_ptr());
        if res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("PowerSetActiveScheme failed: {}", res))
        }
    }
}
