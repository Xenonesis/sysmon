//! Native privilege checks and authenticated, continuous single-instance handoff.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationOutcome {
    Ready,
    Canceled,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum StartupMessage {
    Elevate,
    Install {
        installer: std::path::PathBuf,
        target: std::path::PathBuf,
        sha256: String,
    },
    Resume(crate::updater::InstallOutcome, std::path::PathBuf),
}

#[cfg(windows)]
pub(crate) use native::{complete_startup, initialize_instance, launch_successor, random_id, take_startup_message};

#[cfg(windows)]
pub fn is_app_elevated() -> bool {
    use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let _token = native::Handle(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size,
            &mut size,
        )
        .is_ok()
            && elevation.TokenIsElevated != 0
    }
}

#[cfg(windows)]
pub fn relaunch_as_admin() -> Result<ElevationOutcome, String> {
    if is_app_elevated() {
        return Err("System Monitor already has administrator privileges".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    launch_successor(&exe, &StartupMessage::Elevate, true)
}

#[cfg(not(windows))]
pub fn is_app_elevated() -> bool {
    false
}
#[cfg(not(windows))]
pub fn relaunch_as_admin() -> Result<ElevationOutcome, String> {
    Err("Elevation is only supported on Windows".into())
}

#[cfg(windows)]
mod native {
    use super::{ElevationOutcome, StartupMessage};
    use std::{
        cell::RefCell,
        fs::File,
        io::{Read, Write},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            io::{AsRawHandle, FromRawHandle},
        },
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    use windows::{
        Win32::{
            Foundation::*,
            Storage::FileSystem::*,
            System::{Pipes::*, Threading::*},
        },
        core::{PCWSTR, PWSTR, w},
    };

    pub(super) struct Handle(pub HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    struct Startup {
        // Existence, not thread ownership, enforces one instance. A successor opens
        // this SAME object before readiness, so no release/recreate race exists.
        _instance: Handle,
        channel: Option<(File, Handle)>,
        message: Option<StartupMessage>,
    }
    thread_local! { static STARTUP: RefCell<Option<Startup>> = const { RefCell::new(None) }; }
    const INSTANCE: PCWSTR = w!("Global\\SystemMonitorSingleInstance");
    const TIMEOUT: Duration = Duration::from_secs(60);

    pub(crate) fn random_id() -> Result<String, String> {
        use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};
        let mut bytes = [0u8; 16];
        unsafe {
            BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
                .ok()
                .map_err(|e| e.to_string())?;
        }
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }
    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }
    fn raw(file: &File) -> HANDLE {
        HANDLE(file.as_raw_handle())
    }
    fn birth(process: HANDLE) -> Result<u64, String> {
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        unsafe {
            GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user).map_err(|e| e.to_string())?;
        }
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }
    fn image(process: HANDLE) -> Result<PathBuf, String> {
        let mut buffer = vec![0u16; 32768];
        let mut size = buffer.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size)
                .map_err(|e| e.to_string())?;
        }
        std::fs::canonicalize(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..size as usize])))
            .map_err(|e| e.to_string())
    }
    fn wait_bytes(file: &File, peer: HANDLE, count: u32) -> Result<(), String> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let mut available = 0;
            unsafe {
                PeekNamedPipe(raw(file), None, 0, None, Some(&mut available), None).map_err(|e| e.to_string())?;
            }
            if available >= count {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("Successor readiness timed out; original application remains open".into());
            }
            if unsafe { WaitForSingleObject(peer, 0) } != WAIT_TIMEOUT {
                return Err("Handoff peer exited before readiness".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn send_message(file: &mut File, message: &StartupMessage) -> Result<(), String> {
        let bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
        if bytes.len() > 4096 {
            return Err("Handoff message exceeds limit".into());
        }
        file.write_all(&(bytes.len() as u32).to_le_bytes())
            .and_then(|_| file.write_all(&bytes))
            .map_err(|e| e.to_string())
    }
    fn receive_message(file: &mut File, parent: HANDLE) -> Result<StartupMessage, String> {
        wait_bytes(file, parent, 4)?;
        let mut length = [0; 4];
        file.read_exact(&mut length).map_err(|e| e.to_string())?;
        let length = u32::from_le_bytes(length);
        if length > 4096 {
            return Err("Invalid handoff message length".into());
        }
        wait_bytes(file, parent, length)?;
        let mut bytes = vec![0; length as usize];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    pub(crate) fn initialize_instance() -> Result<(), String> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let startup = if args.first().is_some_and(|a| a == "--sysmon-handoff") {
            let (pid, created, nonce) = parse_handoff(&args)?;
            let parent = Handle(unsafe {
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid)
                    .map_err(|e| e.to_string())?
            });
            if birth(parent.0)? != created {
                return Err("Handoff parent identity changed".into());
            }
            let pipe_name = wide(std::ffi::OsStr::new(&format!(r"\\.\pipe\SysMonHandoff-{nonce}")));
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(pipe_name.as_ptr()),
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_MODE(0),
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
                .map_err(|e| e.to_string())?
            };
            let mut pipe = unsafe { File::from_raw_handle(handle.0) };
            let mut server = 0;
            unsafe {
                GetNamedPipeServerProcessId(raw(&pipe), &mut server).map_err(|e| e.to_string())?;
            }
            if server != pid {
                return Err("Handoff pipe does not belong to the expected parent".into());
            }
            let parent_image = image(parent.0)?;
            let own_image = std::fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if parent_image != own_image {
                // Copied update helpers and newly installed versions must retain
                // the independently configured publisher identity.
                crate::updater::verify_authenticode_path(&parent_image)?;
            }
            pipe.write_all(&[1]).map_err(|e| e.to_string())?;
            let message = receive_message(&mut pipe, parent.0)?;
            if matches!(message, StartupMessage::Elevate | StartupMessage::Install { .. }) && !super::is_app_elevated()
            {
                return Err("Successor did not receive administrator privileges".into());
            }
            let instance =
                Handle(unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, INSTANCE).map_err(|e| e.to_string())? });
            Startup {
                _instance: instance,
                channel: Some((pipe, parent)),
                message: Some(message),
            }
        } else {
            if !args.is_empty() {
                return Err("Unrecognized startup arguments".into());
            }
            let handle = unsafe { CreateMutexW(None, false, INSTANCE).map_err(|e| e.to_string())? };
            let error = unsafe { GetLastError() };
            let instance = Handle(handle);
            if error == ERROR_ALREADY_EXISTS {
                return Err("System Monitor is already running. Check your system tray or taskbar.".into());
            }
            Startup {
                _instance: instance,
                channel: None,
                message: None,
            }
        };
        STARTUP.with(|state| *state.borrow_mut() = Some(startup));
        Ok(())
    }

    fn parse_handoff(args: &[String]) -> Result<(u32, u64, &str), String> {
        if args.len() != 4 || args[3].len() != 32 || !args[3].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Malformed handoff arguments".into());
        }
        let pid = args[1].parse::<u32>().map_err(|e| e.to_string())?;
        let created = args[2].parse::<u64>().map_err(|e| e.to_string())?;
        if pid == 0 || created == 0 {
            return Err("Invalid handoff identity".into());
        }
        Ok((pid, created, &args[3]))
    }

    pub(crate) fn take_startup_message() -> Option<StartupMessage> {
        STARTUP.with(|state| state.borrow_mut().as_mut().and_then(|s| s.message.take()))
    }

    /// Call only once the successor can run (first GUI callback, or verified helper).
    /// The original receives readiness first; it chooses graceful exit. Until it
    /// actually exits, the successor never exposes a second active UI/installer.
    pub(crate) fn complete_startup() -> Result<(), String> {
        let channel = STARTUP.with(|state| state.borrow_mut().as_mut().and_then(|s| s.channel.take()));
        if let Some((mut pipe, parent)) = channel {
            pipe.write_all(&[2]).map_err(|e| e.to_string())?;
            if unsafe { WaitForSingleObject(parent.0, TIMEOUT.as_millis() as u32) } != WAIT_OBJECT_0 {
                return Err("Original application did not exit; successor will not start".into());
            }
        }
        Ok(())
    }

    pub(crate) fn launch_successor(
        path: &Path,
        message: &StartupMessage,
        elevated: bool,
    ) -> Result<ElevationOutcome, String> {
        use windows::Win32::UI::{
            Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_SHOW,
        };
        let nonce = random_id()?;
        let pipe_name = wide(std::ffi::OsStr::new(&format!(r"\\.\pipe\SysMonHandoff-{nonce}")));
        // FIRST_PIPE_INSTANCE refuses pre-created objects; PID checks authenticate
        // each endpoint even if another same-user process learns the random name.
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(pipe_name.as_ptr()),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                8192,
                8192,
                0,
                None,
            )
        };
        if handle.is_invalid() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut pipe = unsafe { File::from_raw_handle(handle.0) };
        let args = format!(
            "--sysmon-handoff {} {} {nonce}",
            std::process::id(),
            birth(unsafe { GetCurrentProcess() })?
        );
        let process = if elevated {
            let path_w = wide(path.as_os_str());
            let args_w = wide(std::ffi::OsStr::new(&args));
            let mut info = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
                lpVerb: w!("runas"),
                lpFile: PCWSTR(path_w.as_ptr()),
                lpParameters: PCWSTR(args_w.as_ptr()),
                nShow: SW_SHOW.0,
                ..Default::default()
            };
            if let Err(error) = unsafe { ShellExecuteExW(&mut info) } {
                if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                    return Ok(ElevationOutcome::Canceled);
                }
                return Err(format!("Could not launch elevated successor: {error}"));
            }
            if info.hProcess.is_invalid() {
                return Err("Windows returned no successor process handle".into());
            }
            Handle(info.hProcess)
        } else {
            use std::os::windows::io::IntoRawHandle;
            let child = std::process::Command::new(path)
                .args(args.split_ascii_whitespace())
                .spawn()
                .map_err(|e| e.to_string())?;
            Handle(HANDLE(child.into_raw_handle()))
        };
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let connected = unsafe { ConnectNamedPipe(raw(&pipe), None) };
            let mut client = 0;
            if unsafe { GetNamedPipeClientProcessId(raw(&pipe), &mut client) }.is_ok() {
                if client != unsafe { GetProcessId(process.0) } {
                    return Err("Unexpected handoff client process".into());
                }
                break;
            }
            let _ = connected; // PIPE_LISTENING is expected for a nonblocking server.
            if Instant::now() >= deadline || unsafe { WaitForSingleObject(process.0, 0) } != WAIT_TIMEOUT {
                return Err("Successor did not connect; original application remains open".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        wait_bytes(&pipe, process.0, 1)?;
        let mut hello = [0];
        pipe.read_exact(&mut hello).map_err(|e| e.to_string())?;
        if hello != [1] {
            return Err("Invalid successor handshake".into());
        }
        send_message(&mut pipe, message)?;
        wait_bytes(&pipe, process.0, 1)?;
        pipe.read_exact(&mut hello).map_err(|e| e.to_string())?;
        if hello != [2] {
            return Err("Successor did not confirm readiness".into());
        }
        Ok(ElevationOutcome::Ready)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn handoff_rejects_missing_identity_or_injected_pipe_path() {
            for args in [
                vec!["--sysmon-handoff", "0", "1", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
                vec!["--sysmon-handoff", "42", "0", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
                vec!["--sysmon-handoff", "42", "1", "..\\attacker"],
                vec!["--sysmon-handoff", "42", "1"],
            ] {
                assert!(parse_handoff(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
            }
        }
    }
}
