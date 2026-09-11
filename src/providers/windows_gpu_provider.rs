//! Windows GPU counters are attributed by adapter LUID, never by row order.
use super::{MetricValue, ProviderData, ProviderError, TelemetryProvider};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
pub struct WindowsGpuProvider {
    available: bool,
}
impl WindowsGpuProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

fn instance_parts(name: &str) -> Option<(String, Option<u32>, Option<String>)> {
    let parts: Vec<_> = name.split('_').collect();
    let luid = parts.iter().position(|p| *p == "luid")?;
    let high = u32::from_str_radix(parts.get(luid + 1)?.trim_start_matches("0x"), 16).ok()?;
    let low = u32::from_str_radix(parts.get(luid + 2)?.trim_start_matches("0x"), 16).ok()?;
    let pid = parts
        .iter()
        .position(|p| *p == "pid")
        .and_then(|i| parts.get(i + 1)?.parse().ok());
    let engine = parts
        .iter()
        .position(|p| *p == "eng")
        .and_then(|i| parts.get(i + 1))
        .map(|v| v.to_string());
    Some((format!("{high:08x}_{low:08x}"), pid, engine))
}

fn engine_usage(rows: impl IntoIterator<Item = (String, f64)>) -> HashMap<String, f64> {
    let mut engines: HashMap<(String, String), f64> = HashMap::new();
    for (name, value) in rows {
        if !value.is_finite() || value < 0.0 {
            continue;
        }
        if let Some((luid, _, Some(engine))) = instance_parts(&name) {
            *engines.entry((luid, engine)).or_default() += value;
        }
    }
    let mut adapters: HashMap<String, f64> = HashMap::new();
    for ((luid, _), usage) in engines {
        let value = adapters.entry(luid).or_default();
        *value = value.max(usage.min(100.0));
    }
    adapters
}

#[cfg(target_os = "windows")]
pub(crate) fn numeric(value: &wmi::Variant) -> Option<f64> {
    let number = match value {
        wmi::Variant::UI1(v) => *v as f64,
        wmi::Variant::UI2(v) => *v as f64,
        wmi::Variant::UI4(v) => *v as f64,
        wmi::Variant::UI8(v) => *v as f64,
        wmi::Variant::I1(v) => *v as f64,
        wmi::Variant::I2(v) => *v as f64,
        wmi::Variant::I4(v) => *v as f64,
        wmi::Variant::I8(v) => *v as f64,
        wmi::Variant::R4(v) => *v as f64,
        wmi::Variant::R8(v) => *v,
        wmi::Variant::String(v) => v.parse().ok()?,
        _ => return None,
    };
    (number.is_finite() && number >= 0.0).then_some(number)
}

#[cfg(target_os = "windows")]
fn counter_rows(
    connection: &wmi::WMIConnection,
    class: &str,
    fields: &str,
) -> Result<Vec<HashMap<String, wmi::Variant>>, String> {
    let mut errors = Vec::new();
    for prefix in ["GPUPerformanceCounters", "GPUPerformanceMonitors"] {
        match connection.raw_query(format!(
            "SELECT Name, {fields} FROM Win32_PerfFormattedData_{prefix}_{class}"
        )) {
            Ok(rows) => return Ok(rows),
            Err(error) => errors.push(error.to_string()),
        }
    }
    Err(errors.join("; "))
}

impl TelemetryProvider for WindowsGpuProvider {
    fn name(&self) -> &str {
        "windows_gpu"
    }
    fn poll_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn is_available(&self) -> bool {
        self.available
    }
    fn poll(&mut self) -> Result<ProviderData, ProviderError> {
        #[cfg(not(target_os = "windows"))]
        {
            Err(ProviderError::Unavailable(
                "Windows GPU counters require Windows".into(),
            ))
        }
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIFactory1};
            let connection = wmi::WMIConnection::new().map_err(|e| ProviderError::InitFailed(e.to_string()))?;
            let factory: IDXGIFactory1 =
                unsafe { CreateDXGIFactory1() }.map_err(|e| ProviderError::InitFailed(e.to_string()))?;
            let rows =
                counter_rows(&connection, "GPUEngine", "UtilizationPercentage").map_err(ProviderError::PollFailed)?;
            let usage = engine_usage(rows.into_iter().filter_map(|row| {
                let wmi::Variant::String(name) = row.get("Name")? else {
                    return None;
                };
                Some((name.clone(), numeric(row.get("UtilizationPercentage")?)?))
            }));
            let memory = counter_rows(&connection, "GPULocalAdapterMemory", "LocalUsage");
            let mut used: HashMap<String, u64> = HashMap::new();
            if let Ok(rows) = &memory {
                for row in rows {
                    let Some(wmi::Variant::String(name)) = row.get("Name") else {
                        continue;
                    };
                    if let Some((luid, _, _)) = instance_parts(name)
                        && let Some(value) = row.get("LocalUsage").and_then(numeric)
                    {
                        *used.entry(luid).or_default() += value as u64;
                    }
                }
            }
            let mut data = ProviderData::new();
            let mut index = 0;
            loop {
                let adapter = match unsafe { factory.EnumAdapters1(index) } {
                    Ok(adapter) => adapter,
                    Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(error) => return Err(ProviderError::PollFailed(error.to_string())),
                };
                index += 1;
                let desc = unsafe { adapter.GetDesc1() }.map_err(|e| ProviderError::PollFailed(e.to_string()))?;
                let luid = format!(
                    "{:08x}_{:08x}",
                    desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart
                );
                let prefix = format!("gpu.windows.{luid}");
                let length = desc
                    .Description
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(desc.Description.len());
                data.insert(
                    format!("{prefix}.name"),
                    MetricValue::Text(String::from_utf16_lossy(&desc.Description[..length])),
                );
                if let Some(value) = usage.get(&luid) {
                    data.insert(format!("{prefix}.utilization"), MetricValue::Float(*value));
                }
                if let Some(value) = used.get(&luid) {
                    data.insert(format!("{prefix}.vram_used"), MetricValue::UInt(*value));
                }
                data.insert(
                    format!("{prefix}.vram_total"),
                    MetricValue::UInt(desc.DedicatedVideoMemory as u64),
                );
                if let Err(error) = &memory {
                    data.insert(format!("{prefix}.memory_error"), MetricValue::Text(error.clone()));
                }
            }
            self.available = true;
            Ok(data)
        }
    }
}

/// Bracket WDDM acquisition with native creation identities: PID reuse cannot inherit a reading.
pub(crate) fn query_process_memory() -> Result<HashMap<crate::processes::ProcessIdentity, u64>, String> {
    #[cfg(not(target_os = "windows"))]
    {
        Err("WDDM process memory is unavailable on this platform".into())
    }
    #[cfg(target_os = "windows")]
    {
        let connection = wmi::WMIConnection::new().map_err(|e| e.to_string())?;
        let first = counter_rows(&connection, "GPUProcessMemory", "DedicatedUsage")?;
        let identities: HashMap<_, _> = first
            .iter()
            .filter_map(|row| {
                let wmi::Variant::String(name) = row.get("Name")? else {
                    return None;
                };
                let (_, Some(pid), _) = instance_parts(name)? else {
                    return None;
                };
                crate::processes::process_identity(pid)
                    .ok()
                    .map(|identity| (pid, identity))
            })
            .collect();
        let rows = counter_rows(&connection, "GPUProcessMemory", "DedicatedUsage")?;
        let mut per_adapter: HashMap<(crate::processes::ProcessIdentity, String), u64> = HashMap::new();
        for row in rows {
            let Some(wmi::Variant::String(name)) = row.get("Name") else {
                continue;
            };
            let Some((luid, Some(pid), _)) = instance_parts(name) else {
                continue;
            };
            let Some(identity) = identities.get(&pid) else { continue };
            if crate::processes::process_identity(pid).ok().as_ref() != Some(identity) {
                continue;
            }
            if let Some(value) = row.get("DedicatedUsage").and_then(numeric) {
                per_adapter
                    .entry((*identity, luid))
                    .and_modify(|v| *v = (*v).max(value as u64))
                    .or_insert(value as u64);
            }
        }
        let mut result: HashMap<crate::processes::ProcessIdentity, u64> = HashMap::new();
        for ((identity, _), bytes) in per_adapter {
            *result.entry(identity).or_default() += bytes;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapters_do_not_cross_assign_and_engine_processes_sum() {
        let values = engine_usage([
            ("pid_1_luid_0x00000000_0x00000001_phys_0_eng_0_engtype_3D".into(), 30.0),
            ("pid_2_luid_0x00000000_0x00000001_phys_0_eng_0_engtype_3D".into(), 40.0),
            ("pid_3_luid_0x00000000_0x00000002_phys_0_eng_0_engtype_3D".into(), 5.0),
        ]);
        assert_eq!(values["00000000_00000001"], 70.0);
        assert_eq!(values["00000000_00000002"], 5.0);
    }
}
