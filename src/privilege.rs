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

#[cfg(not(target_os = "windows"))]
pub fn is_app_elevated() -> bool {
    false
}
