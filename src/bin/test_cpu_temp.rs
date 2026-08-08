use wmi::{COMLibrary, WMIConnection};
use std::rc::Rc;

fn main() {
    let com = COMLibrary::new().expect("COM init failed");
    let wmi = WMIConnection::with_namespace_path("ROOT\\WMI", Rc::new(com)).expect("WMI init failed");
    let results: Vec<std::collections::HashMap<String, wmi::Variant>> = wmi
        .raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature")
        .expect("query failed");
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