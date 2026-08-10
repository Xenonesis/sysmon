//! Privilege and elevation checks for Windows.

#[cfg(target_os = "windows")]
pub fn is_app_elevated() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use std::mem;

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;
            let res = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size,
                &mut size,
            );
            let _ = CloseHandle(token);
            if res.is_ok() {
                return elevation.TokenIsElevated != 0;
            }
        }
    }
    false
}

#[cfg(target_os = "windows")]
pub fn relaunch_as_admin() -> bool {
    use std::ptr;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteExW;
    use windows::Win32::UI::Shell::SHELLEXECUTEINFOW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    if let Ok(path) = std::env::current_exe() {
        let mut path_w: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();
        
        let mut info = SHELLEXECUTEINFOW::default();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.lpVerb = windows::core::PCWSTR(verb.as_ptr());
        info.lpFile = windows::core::PCWSTR(path_w.as_ptr());
        info.nShow = SW_SHOW.0 as i32;

        unsafe {
            if ShellExecuteExW(&mut info).is_ok() {
                std::process::exit(0);
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
