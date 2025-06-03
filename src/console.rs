#[cfg(target_os = "windows")]
use windows::Win32::{
    UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR, ShowWindow, SW_HIDE, SW_SHOW},
    System::Console::{AllocConsole, GetConsoleWindow, SetConsoleTitleW},
};

// Function to show error dialog on Windows
#[cfg(target_os = "windows")]
pub fn show_error_dialog(message: &str) {
    use windows::core::PCWSTR;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    let wide_title: Vec<u16> = OsStr::new("Spotify Server Error")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_message: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
      unsafe {
        MessageBoxW(
            None,
            PCWSTR(wide_message.as_ptr()),
            PCWSTR(wide_title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_error_dialog(message: &str) {
    eprintln!("Error: {}", message);
}

// Function to hide console window on Windows
#[cfg(target_os = "windows")]
pub fn hide_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if !console_window.is_invalid() {
            let _ = ShowWindow(console_window, SW_HIDE);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn hide_console() {
    // No-op on non-Windows platforms
}

// Function to show console window on Windows
#[cfg(target_os = "windows")]
pub fn show_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window.is_invalid() {
            // Allocate a new console if one doesn't exist (Windows subsystem mode)
            if AllocConsole().is_ok() {
                // Set console title
                use windows::core::PCWSTR;
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                
                let title: Vec<u16> = OsStr::new("Spotify Server - Debug Console")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = SetConsoleTitleW(PCWSTR(title.as_ptr()));
                
                // Redirect stdout, stderr, and stdin to the new console
                redirect_console_streams();
            }
        } else {
            // Console already exists, just show it
            let _ = ShowWindow(console_window, SW_SHOW);
        }
    }
}

#[cfg(target_os = "windows")]
fn redirect_console_streams() {
    use std::ffi::CString;
    
    unsafe {
        // Redirect stdout to console
        if let Ok(conout) = CString::new("CONOUT$") {
            if let Ok(mode) = CString::new("w") {
                libc::freopen(conout.as_ptr(), mode.as_ptr(), libc_stdhandle::stdout());
            }
        }
        
        // Redirect stderr to console
        if let Ok(conout) = CString::new("CONOUT$") {
            if let Ok(mode) = CString::new("w") {
                libc::freopen(conout.as_ptr(), mode.as_ptr(), libc_stdhandle::stderr());
            }
        }
        
        // Redirect stdin to console
        if let Ok(conin) = CString::new("CONIN$") {
            if let Ok(mode) = CString::new("r") {
                libc::freopen(conin.as_ptr(), mode.as_ptr(), libc_stdhandle::stdin());
            }
        }
    }
}

// Helper module to get standard handles as FILE pointers
#[cfg(target_os = "windows")]
mod libc_stdhandle {
    use libc::FILE;
    
    extern "C" {
        #[link_name = "__acrt_iob_func"]
        fn acrt_iob_func(fd: u32) -> *mut FILE;
    }
    
    pub unsafe fn stdin() -> *mut FILE {
        acrt_iob_func(0)
    }
    
    pub unsafe fn stdout() -> *mut FILE {
        acrt_iob_func(1)
    }
    
    pub unsafe fn stderr() -> *mut FILE {
        acrt_iob_func(2)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_console() {
    // No-op on non-Windows platforms
}
