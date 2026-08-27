use serde::Deserialize;
use wmi::WMIConnection;

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types, dead_code)]
struct Win32_Service {
    name: String,
    display_name: Option<String>,
    state: String,
}

fn main() {
    let wmi_con = match WMIConnection::new() {
        Ok(con) => con,
        Err(e) => {
            eprintln!("Error: WMI connection failed: {:?}", e);
            std::process::exit(1);
        }
    };
    let results: Result<Vec<Win32_Service>, _> =
        wmi_con.raw_query("SELECT Name, DisplayName, State FROM Win32_Service");
    match results {
        Ok(v) => println!("Success: {}", v.len()),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
