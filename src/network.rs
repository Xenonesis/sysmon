//! Active TCP and UDP socket connection monitoring with process PID resolution.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SocketConnection {
    pub protocol: &'static str,
    pub local_addr: String,
    pub remote_addr: String,
    pub state: &'static str,
    pub pid: u32,
    pub process_name: Option<String>,
}

/// Parse an IPv6 address from a 16-byte array and a port in network byte order in lower 16 bits.
pub fn parse_ipv6_addr(bytes: &[u8; 16], port_raw: u32) -> String {
    let ip = Ipv6Addr::from(*bytes);
    let port = u16::from_be((port_raw & 0xFFFF) as u16);
    format!("[{ip}]:{port}")
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;

    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
    const UDP_TABLE_OWNER_PID: u32 = 1;
    const AF_INET6: u32 = 23;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct MIB_TCPROW_OWNER_PID {
        dw_state: u32,
        dw_local_addr: u32,
        dw_local_port: u32,
        dw_remote_addr: u32,
        dw_remote_port: u32,
        dw_owning_pid: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct MIB_UDPROW_OWNER_PID {
        dw_local_addr: u32,
        dw_local_port: u32,
        dw_owning_pid: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct MIB_TCP6ROW_OWNER_PID {
        uc_local_addr: [u8; 16],
        dw_local_scope_id: u32,
        dw_local_port: u32,
        uc_remote_addr: [u8; 16],
        dw_remote_scope_id: u32,
        dw_remote_port: u32,
        dw_state: u32,
        dw_owning_pid: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct MIB_UDP6ROW_OWNER_PID {
        uc_local_addr: [u8; 16],
        dw_local_scope_id: u32,
        dw_local_port: u32,
        dw_owning_pid: u32,
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetExtendedTcpTable(
            pTcpTable: *mut u8,
            pdwSize: *mut u32,
            bOrder: i32,
            ulAf: u32,
            TableClass: u32,
            Reserved: u32,
        ) -> u32;

        fn GetExtendedUdpTable(
            pUdpTable: *mut u8,
            pdwSize: *mut u32,
            bOrder: i32,
            ulAf: u32,
            TableClass: u32,
            Reserved: u32,
        ) -> u32;
    }

    fn tcp_state_str(state: u32) -> &'static str {
        match state {
            1 => "CLOSED",
            2 => "LISTEN",
            3 => "SYN_SENT",
            4 => "SYN_RCVD",
            5 => "ESTABLISHED",
            6 => "FIN_WAIT1",
            7 => "FIN_WAIT2",
            8 => "CLOSE_WAIT",
            9 => "CLOSING",
            10 => "LAST_ACK",
            11 => "TIME_WAIT",
            12 => "DELETE_TCB",
            _ => "UNKNOWN",
        }
    }

    fn parse_ipv4(addr_raw: u32, port_raw: u32) -> String {
        let ip = Ipv4Addr::from(addr_raw.to_le());
        // Ports in MIB rows are in network byte order in the lower 16 bits
        let port = u16::from_be((port_raw & 0xFFFF) as u16);
        format!("{ip}:{port}")
    }

    pub fn get_connections_internal() -> Vec<SocketConnection> {
        let mut connections = Vec::new();

        // 1. Fetch TCP Table
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, AF_INET, TCP_TABLE_OWNER_PID_ALL, 0);
            if size > 0 {
                let mut buffer = vec![0u8; size as usize];
                if GetExtendedTcpTable(buffer.as_mut_ptr(), &mut size, 0, AF_INET, TCP_TABLE_OWNER_PID_ALL, 0) == 0 {
                    let num_entries = *(buffer.as_ptr() as *const u32) as usize;
                    let table_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_TCPROW_OWNER_PID;
                    for i in 0..num_entries {
                        let row = *table_ptr.add(i);
                        let local = parse_ipv4(row.dw_local_addr, row.dw_local_port);
                        let remote = if row.dw_remote_addr == 0 {
                            "*:*".to_string()
                        } else {
                            parse_ipv4(row.dw_remote_addr, row.dw_remote_port)
                        };

                        connections.push(SocketConnection {
                            protocol: "TCP",
                            local_addr: local,
                            remote_addr: remote,
                            state: tcp_state_str(row.dw_state),
                            pid: row.dw_owning_pid,
                            process_name: None,
                        });
                    }
                }
            }
        }

        // 2. Fetch UDP Table
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedUdpTable(std::ptr::null_mut(), &mut size, 0, AF_INET, UDP_TABLE_OWNER_PID, 0);
            if size > 0 {
                let mut buffer = vec![0u8; size as usize];
                if GetExtendedUdpTable(buffer.as_mut_ptr(), &mut size, 0, AF_INET, UDP_TABLE_OWNER_PID, 0) == 0 {
                    let num_entries = *(buffer.as_ptr() as *const u32) as usize;
                    let table_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_UDPROW_OWNER_PID;
                    for i in 0..num_entries {
                        let row = *table_ptr.add(i);
                        let local = parse_ipv4(row.dw_local_addr, row.dw_local_port);

                        connections.push(SocketConnection {
                            protocol: "UDP",
                            local_addr: local,
                            remote_addr: "*:*".to_string(),
                            state: "LISTEN",
                            pid: row.dw_owning_pid,
                            process_name: None,
                        });
                    }
                }
            }
        }

        // 3. Fetch IPv6 TCP Table
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, AF_INET6, TCP_TABLE_OWNER_PID_ALL, 0);
            if size > 0 {
                let mut buffer = vec![0u8; size as usize];
                if GetExtendedTcpTable(buffer.as_mut_ptr(), &mut size, 0, AF_INET6, TCP_TABLE_OWNER_PID_ALL, 0) == 0 {
                    let num_entries = *(buffer.as_ptr() as *const u32) as usize;
                    let table_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_TCP6ROW_OWNER_PID;
                    for i in 0..num_entries {
                        let row = *table_ptr.add(i);
                        let local = parse_ipv6_addr(&row.uc_local_addr, row.dw_local_port);
                        let remote = if row.uc_remote_addr == [0u8; 16] {
                            "[::]:*".to_string()
                        } else {
                            parse_ipv6_addr(&row.uc_remote_addr, row.dw_remote_port)
                        };

                        connections.push(SocketConnection {
                            protocol: "TCPv6",
                            local_addr: local,
                            remote_addr: remote,
                            state: tcp_state_str(row.dw_state),
                            pid: row.dw_owning_pid,
                            process_name: None,
                        });
                    }
                }
            }
        }

        // 4. Fetch IPv6 UDP Table
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedUdpTable(std::ptr::null_mut(), &mut size, 0, AF_INET6, UDP_TABLE_OWNER_PID, 0);
            if size > 0 {
                let mut buffer = vec![0u8; size as usize];
                if GetExtendedUdpTable(buffer.as_mut_ptr(), &mut size, 0, AF_INET6, UDP_TABLE_OWNER_PID, 0) == 0 {
                    let num_entries = *(buffer.as_ptr() as *const u32) as usize;
                    let table_ptr = buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MIB_UDP6ROW_OWNER_PID;
                    for i in 0..num_entries {
                        let row = *table_ptr.add(i);
                        let local = parse_ipv6_addr(&row.uc_local_addr, row.dw_local_port);

                        connections.push(SocketConnection {
                            protocol: "UDPv6",
                            local_addr: local,
                            remote_addr: "[::]:*".to_string(),
                            state: "LISTEN",
                            pid: row.dw_owning_pid,
                            process_name: None,
                        });
                    }
                }
            }
        }

        connections
    }
}

/// Retrieve active system socket connections with process names resolved from `process_names` map.
pub fn get_active_connections(process_names: &HashMap<u32, String>) -> Vec<SocketConnection> {
    #[cfg(target_os = "windows")]
    let mut connections = windows_impl::get_connections_internal();

    #[cfg(not(target_os = "windows"))]
    let mut connections: Vec<SocketConnection> = Vec::new();

    for conn in &mut connections {
        if let Some(name) = process_names.get(&conn.pid) {
            conn.process_name = Some(name.clone());
        }
    }

    connections
}

/// Filter socket connections by search substring (PID, IP, Port, Process name, or State).
pub fn filter_connections(items: &[SocketConnection], query: &str) -> Vec<SocketConnection> {
    if query.trim().is_empty() {
        return items.to_vec();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|c| {
            c.protocol.to_lowercase().contains(&q)
                || c.local_addr.to_lowercase().contains(&q)
                || c.remote_addr.to_lowercase().contains(&q)
                || c.state.to_lowercase().contains(&q)
                || c.pid.to_string().contains(&q)
                || c.process_name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_connections() {
        let items = vec![
            SocketConnection {
                protocol: "TCP",
                local_addr: "127.0.0.1:8080".into(),
                remote_addr: "0.0.0.0:0".into(),
                state: "LISTEN",
                pid: 1234,
                process_name: Some("server.exe".into()),
            },
            SocketConnection {
                protocol: "UDP",
                local_addr: "0.0.0.0:53".into(),
                remote_addr: "*:*".into(),
                state: "LISTEN",
                pid: 5678,
                process_name: Some("dns.exe".into()),
            },
        ];

        assert_eq!(filter_connections(&items, "8080").len(), 1);
        assert_eq!(filter_connections(&items, "server").len(), 1);
        assert_eq!(filter_connections(&items, "UDP").len(), 1);
        assert_eq!(filter_connections(&items, "").len(), 2);
    }

    #[test]
    fn test_parse_ipv6_formatting() {
        let raw_bytes: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01];
        let formatted = parse_ipv6_addr(&raw_bytes, 443u32.to_be() << 16);
        assert!(formatted.contains("[2001:db8::1]"));
        // port 443 in network byte order in lower 16 bits
        let formatted_direct = parse_ipv6_addr(&raw_bytes, 443);
        assert!(formatted_direct.contains("[2001:db8::1]"));

        // Test loopback IPv6 and port 80 in network byte order
        let loopback = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let port_80_nbo = 80u16.to_be() as u32;
        assert_eq!(parse_ipv6_addr(&loopback, port_80_nbo), "[::1]:80");
    }
}
