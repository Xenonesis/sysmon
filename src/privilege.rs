//! Privilege and elevation checks for Windows.

#[cfg(target_os = "windows")]
pub fn is_app_elevated() -> bool {
    use std::mem;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    struct HandleGuard(windows::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let _guard = HandleGuard(token);
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;
            let res = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size,
                &mut size,
            );
            if res.is_ok() {
                return elevation.TokenIsElevated != 0;
            }
        }
    }
    false
}

#[cfg(target_os = "windows")]
pub fn relaunch_as_admin() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteExW;
    use windows::Win32::UI::Shell::SHELLEXECUTEINFOW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    if let Ok(path) = std::env::current_exe() {
        let path_w: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();

        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            lpVerb: windows::core::PCWSTR(verb.as_ptr()),
            lpFile: windows::core::PCWSTR(path_w.as_ptr()),
            nShow: SW_SHOW.0,
            ..Default::default()
        };

        unsafe {
            if ShellExecuteExW(&mut info).is_ok() {
                // Return true, let caller handle graceful exit (drop app state) instead of hard exit
                return true;
            }
        }
    }
    false
}
#[cfg(not(target_os = "windows"))]
pub fn is_app_elevated() -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn relaunch_as_admin() -> bool {
    false
}
