use serde::Deserialize;
use wmi::{WMIConnection, COMLibrary};

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types, dead_code)]
struct Win32_Service {
    name: String,
    display_name: Option<String>,
    state: String,
}

fn main() {
    let com_lib = COMLibrary::new().unwrap();
    let wmi_con = WMIConnection::new(com_lib.into()).unwrap();
    let results: Result<Vec<Win32_Service>, _> = wmi_con.raw_query("SELECT Name, DisplayName, State FROM Win32_Service");
    match results {
        Ok(v) => println!("Success: {}", v.len()),
        Err(e) => println!("Error: {:?}", e),
    }
}
