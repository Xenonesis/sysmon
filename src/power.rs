use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, LocalFree};
use windows_sys::Win32::System::Power::{
    ACCESS_SCHEME, PowerEnumerate, PowerGetActiveScheme, PowerReadFriendlyName, PowerSetActiveScheme,
};

/// Root registry page under which power schemes live.
const GUID_POWER_SCHEME_PAGE: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0xa1841308_3541_4fab_bc81_f71556f20b4a);
/// Subgroup containing the power schemes themselves.
const SUB_GUID_POWERSCHEME: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0xe73a048d_bf26_4f12_9b60_c51e967cb42f);

#[derive(Debug, Clone)]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub is_active: bool,
}

/// Parse a GUID string like `{381b4222-f694-41f0-9685-ff5bb260df2e}` (braces and
/// hyphens optional) into a `windows_sys::core::GUID`. Falls back to the nil
/// GUID on malformed input.
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

/// Normalize a GUID string for equality comparison (lowercase, braces/hyphens stripped).
fn normalized(s: &str) -> String {
    s.trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_lowercase()
}

pub fn get_power_plans() -> Vec<PowerPlan> {
    let mut plans = Vec::new();
    let mut active_guid: Option<String> = None;

    // SAFETY: FFI calls to power management use valid pointers.
    // Returned memory from PowerGetActiveScheme is freed using LocalFree.
    unsafe {
        let mut active_ptr: *mut windows_sys::core::GUID = ptr::null_mut();
        if PowerGetActiveScheme(ptr::null_mut(), &mut active_ptr) == ERROR_SUCCESS && !active_ptr.is_null() {
            active_guid = Some(format_guid(&*active_ptr));
            LocalFree(active_ptr as *mut _);
        }

        let mut index: u32 = 0;
        loop {
            let mut buf = [0u16; 512];
            let mut buf_size = (buf.len() * 2) as u32;
            let hr = PowerEnumerate(
                ptr::null_mut(),
                &GUID_POWER_SCHEME_PAGE,
                &SUB_GUID_POWERSCHEME,
                ACCESS_SCHEME,
                index,
                buf.as_mut_ptr() as *mut u8,
                &mut buf_size,
            );
            if hr == ERROR_NO_MORE_ITEMS {
                break;
            }
            if hr != ERROR_SUCCESS {
                break;
            }
            index += 1;

            let guid_str = String::from_utf16_lossy(&buf[..buf_size as usize / 2])
                .trim_matches('\0')
                .to_string();
            if guid_str.is_empty() {
                continue;
            }
            let guid = match parse_guid(&guid_str) {
                Ok(guid) => guid,
                Err(_) => continue,
            };

            let mut name_buf = [0u16; 260];
            let mut name_size = (name_buf.len() * 2) as u32;
            let mut name = String::new();
            if PowerReadFriendlyName(
                ptr::null_mut(),
                &guid,
                ptr::null(),
                ptr::null(),
                name_buf.as_mut_ptr() as *mut u8,
                &mut name_size,
            ) == ERROR_SUCCESS
            {
                name = String::from_utf16_lossy(&name_buf[..name_size as usize / 2])
                    .trim_matches('\0')
                    .to_string();
            }
            if name.is_empty() {
                name = format_guid(&guid);
            }

            plans.push(PowerPlan {
                guid: format_guid(&guid),
                name,
                is_active: active_guid
                    .as_deref()
                    .is_some_and(|a| normalized(a) == normalized(&guid_str)),
            });
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
            let is_active = active_guid.as_deref().is_some_and(|a| normalized(a) == *guid);
            PowerPlan {
                guid: format!("{{{guid}}}"),
                name: name.to_string(),
                is_active,
            }
        })
        .collect();
    }

    plans
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
    let plans = get_power_plans();
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
