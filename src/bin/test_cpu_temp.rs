use wmi::WMIConnection;

fn main() {
    // Note: MSAcpi_ThermalZoneTemperature lives in ROOT\WMI and typically
    // requires an elevated (administrator) prompt; access denied otherwise.
    let wmi = match WMIConnection::with_namespace_path("ROOT\\WMI") {
        Ok(con) => con,
        Err(e) => {
            eprintln!("Error: WMI init failed: {:?}", e);
            std::process::exit(1);
        }
    };
    let results: Vec<std::collections::HashMap<String, wmi::Variant>> =
        match wmi.raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature") {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("Error: query failed: {:?} (try running as administrator)", e);
                std::process::exit(1);
            }
        };
    println!("ROWS: {}", results.len());
    for (i, row) in results.iter().enumerate() {
        if let Some(val) = row.get("CurrentTemperature") {
            match val {
                wmi::Variant::UI4(n) => {
                    let temp_k_tenths = *n as f32;
                    let temp_c = (temp_k_tenths / 10.0) - 273.15;
                    println!("ROW {}: raw={:?} tenths={} C={:.2}", i, val, temp_k_tenths, temp_c);
                }
                wmi::Variant::I4(n) => {
                    let temp_k_tenths = *n as f32;
                    let temp_c = (temp_k_tenths / 10.0) - 273.15;
                    println!("ROW {}: raw={:?} tenths={} C={:.2}", i, val, temp_k_tenths, temp_c);
                }
                wmi::Variant::UI8(n) => {
                    let temp_k_tenths = *n as f32;
                    let temp_c = (temp_k_tenths / 10.0) - 273.15;
                    println!("ROW {}: raw={:?} tenths={} C={:.2}", i, val, temp_k_tenths, temp_c);
                }
                _ => println!("ROW {}: raw={:?}", i, val),
            }
        }
    }
}
