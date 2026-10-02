use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, LocalFree};
use windows_sys::Win32::System::Power::{
    ACCESS_SCHEME, PowerEnumerate, PowerGetActiveScheme, PowerReadFriendlyName, PowerSetActiveScheme,
};

#[derive(Debug, Clone)]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub is_active: bool,
}

/// Parse a textual GUID, rejecting malformed input.
fn parse_guid(s: &str) -> Result<windows_sys::core::GUID, String> {
    let hex: String = s
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .chars()
        .filter(|c| *c != '-')
        .collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid power-plan GUID: {s}"));
    }
    u128::from_str_radix(&hex, 16)
        .map(windows_sys::core::GUID::from_u128)
        .map_err(|_| format!("Invalid power-plan GUID: {s}"))
}

/// Format a `windows_sys::core::GUID` as a canonical `{xxxxxxxx-xxxx-...}` string.
fn format_guid(g: &windows_sys::core::GUID) -> String {
    format!(
        "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}
pub fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    let mut plans = Vec::new();
    unsafe {
        let mut active_ptr: *mut windows_sys::core::GUID = ptr::null_mut();
        let status = PowerGetActiveScheme(ptr::null_mut(), &mut active_ptr);
        if status != ERROR_SUCCESS || active_ptr.is_null() {
            return Err(format!("PowerGetActiveScheme failed: {status}"));
        }
        let active_guid = format_guid(&*active_ptr);
        LocalFree(active_ptr.cast());
        for index in 0.. {
            let mut guid = windows_sys::core::GUID::from_u128(0);
            let mut size = std::mem::size_of::<windows_sys::core::GUID>() as u32;
            let status = PowerEnumerate(
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ACCESS_SCHEME,
                index,
                (&mut guid as *mut windows_sys::core::GUID).cast(),
                &mut size,
            );
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS || size != std::mem::size_of::<windows_sys::core::GUID>() as u32 {
                return Err(format!("PowerEnumerate({index}) failed: {status}; GUID size {size}"));
            }
            let guid_string = format_guid(&guid);
            let mut name_size = 0;
            let status = PowerReadFriendlyName(
                ptr::null_mut(),
                &guid,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                &mut name_size,
            );
            let mut name = guid_string.clone();
            if (status == ERROR_SUCCESS || status == windows_sys::Win32::Foundation::ERROR_MORE_DATA)
                && name_size > 0
                && name_size % 2 == 0
            {
                let mut buffer = vec![0u16; name_size as usize / 2];
                if PowerReadFriendlyName(
                    ptr::null_mut(),
                    &guid,
                    ptr::null(),
                    ptr::null(),
                    buffer.as_mut_ptr().cast(),
                    &mut name_size,
                ) == ERROR_SUCCESS
                    && name_size as usize / 2 <= buffer.len()
                {
                    name = String::from_utf16_lossy(&buffer[..name_size as usize / 2])
                        .trim_end_matches('\0')
                        .to_string();
                    if name.is_empty() {
                        name = guid_string.clone();
                    }
                }
            }
            plans.push(PowerPlan {
                is_active: guid_string == active_guid,
                guid: guid_string,
                name,
            });
        }
    }
    Ok(plans)
}

pub fn set_active_power_plan(guid: &str) -> Result<(), String> {
    // SAFETY: PowerSetActiveScheme is called with a valid GUID reference.
    unsafe {
        let g = parse_guid(guid)?;
        let res = PowerSetActiveScheme(ptr::null_mut(), &g);
        if res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("PowerSetActiveScheme failed: {res}"))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct BatteryHealth {
    pub has_battery: bool,
    pub percentage: f32,
    pub is_charging: bool,
    pub ac_online: bool,
    pub battery_saver: bool,
    pub full_charge_mwh: Option<u64>,
    pub design_capacity_mwh: Option<u64>,
    pub health_percent: Option<f32>,
    pub cycle_count: Option<u32>,
}

impl BatteryHealth {
    pub fn empty() -> Self {
        Self {
            has_battery: false,
            percentage: 0.0,
            is_charging: false,
            ac_online: true,
            battery_saver: false,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            health_percent: None,
            cycle_count: None,
        }
    }
}

/// Queries system power and battery status natively on Windows.
#[cfg(target_os = "windows")]
pub fn get_battery_health() -> BatteryHealth {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    unsafe {
        let mut sps = std::mem::zeroed::<SYSTEM_POWER_STATUS>();
        if GetSystemPowerStatus(&mut sps) != 0 {
            let has_battery = sps.BatteryFlag != 128 && sps.BatteryLifePercent != 255;
            let percentage = if sps.BatteryLifePercent == 255 {
                0.0
            } else {
                sps.BatteryLifePercent as f32
            };
            let is_charging = (sps.BatteryFlag & 8) != 0;
            let ac_online = sps.ACLineStatus == 1;
            let battery_saver = sps.SystemStatusFlag == 1;

            BatteryHealth {
                has_battery,
                percentage,
                is_charging,
                ac_online,
                battery_saver,
                full_charge_mwh: None,
                design_capacity_mwh: None,
                health_percent: if has_battery { Some(100.0) } else { None },
                cycle_count: None,
            }
        } else {
            BatteryHealth::empty()
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_battery_health() -> BatteryHealth {
    BatteryHealth::empty()
}

/// Activate a power plan matching a mode substring ("balanced", "high performance", "power saver").
#[allow(dead_code)]
pub fn activate_power_mode(mode: &str) -> Result<String, String> {
    let plans = get_power_plans()?;
    let target = mode.to_lowercase();
    if let Some(plan) = plans.iter().find(|p| p.name.to_lowercase().contains(&target)) {
        set_active_power_plan(&plan.guid)?;
        Ok(plan.name.clone())
    } else {
        Err(format!("Power plan matching '{mode}' not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_guid() {
        assert!(parse_guid("not-a-guid").is_err());
        assert!(parse_guid("{00000000-0000-0000-0000-00000000000z}").is_err());
    }

    #[test]
    fn parses_canonical_guid() {
        assert_eq!(
            format_guid(&parse_guid("{381b4222-f694-41f0-9685-ff5bb260df2e}").unwrap()),
            "{381b4222-f694-41f0-9685-ff5bb260df2e}"
        );
    }

    #[test]
    fn battery_health_empty_defaults() {
        let empty = BatteryHealth::empty();
        assert!(!empty.has_battery);
        assert_eq!(empty.percentage, 0.0);
        assert!(empty.ac_online);
        assert!(!empty.is_charging);
    }

    #[test]
    fn get_battery_health_does_not_panic() {
        let health = get_battery_health();
        assert!(health.percentage >= 0.0 && health.percentage <= 100.0);
    }
}
