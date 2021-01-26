use std::env;
use std::ffi::CString;
use std::fmt::Write;
use std::ptr;

use anyhow::{anyhow, Context, Result};
use winapi::shared::winerror;
use winapi::um::shellapi::{self, ShellExecuteA};
use winapi::um::winuser::SW_SHOWNORMAL;

fn help_and_exit() -> ! {
    let msg = "\
        usage: winstart.exe FILE [ARGUMENTS...]\n\
        \n\
        FILE may be a filename, URL, or executable file.\n\
        If FILE is an executable, ARGUMENTS are joined with spaces when passed\n\
        to the the programs Windows-style command-line. Any arguments with spaces\n\
        will be surrounded by double quotes.";
    println!("{}", msg);
    std::process::exit(1);
}

/// Error messages according to
/// https://docs.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutea
fn check_shellexecute_status(status: u32) -> Result<()> {
    if status > 32 {
        Ok(())
    } else {
        let msg = match status {
            0 => " The operating system is out of memory or resources.",
            winerror::ERROR_FILE_NOT_FOUND => "The specified file was not found",
            winerror::ERROR_PATH_NOT_FOUND => "The specified path was not found.",
            winerror::ERROR_BAD_FORMAT => "The .exe file is invalid (non-Win32 .exe or error in .exe image).",
            shellapi::SE_ERR_ACCESSDENIED => "The operating system denied access to the specified file.",
            shellapi::SE_ERR_ASSOCINCOMPLETE => "The file name association is incomplete or invalid.",
            shellapi::SE_ERR_DDEBUSY => "The DDE transaction could not be completed because other DDE transactions were being processed.",
            shellapi::SE_ERR_DDEFAIL => "The DDE transaction failed.",
            shellapi::SE_ERR_DDETIMEOUT => "The DDE transaction could not be completed because the request timed out.",
            shellapi::SE_ERR_DLLNOTFOUND => "The specified DLL was not found.",
            //shellapi::SE_ERR_FNF => "The specified file was not found.",
            shellapi::SE_ERR_NOASSOC => "There is no application associated with the given file name extension.",
            shellapi::SE_ERR_OOM => "There was not enough memory to complete the operation.",
            //shellapi::SE_ERR_PNF => "The specified path was not found.",
            shellapi::SE_ERR_SHARE => "A sharing violation occurred. ",
            _ => "An unknown error occurred.",
        };
        Err(anyhow!("ShellExecute: {}", msg))
    }
}

fn clean_environment() {
    // In MSYS, HOME will be set to the Windows path of the MSYS home directory, which is usually
    // wrong for native windows programs, so clean up the environment a bit.
    // When launched from WSL2 /init, we get a default Windows environment.
    if env::var_os("MSYSTEM").is_some() {
        // there's plenty of other cruft, but these are the most likely to break/confuse programs
        ["HOME", "SHELL", "MSYSTEM"]
            .iter()
            .for_each(env::remove_var);
    }
}

fn run() -> Result<()> {
    let my_args: Vec<_> = env::args().collect();
    let file = my_args.get(1).ok_or_else(|| anyhow!("no file specified"))?;

    match file.as_str() {
        "-h" | "--help" | "/?" => help_and_exit(),
        _ => (),
    };

    let args = if my_args.len() > 2 {
        let mut s = String::new();
        for (i, a) in my_args[2..].iter().enumerate() {
            if i != 0 {
                s.push(' ');
            }
            if a.contains(' ') {
                write!(s, "\"{}\"", a).unwrap();
            } else {
                s.push_str(a)
            }
        }
        Some(s)
    } else {
        None
    };

    let file_c = CString::new(file.as_bytes()).context("invalid filename (contains NULL)")?;
    let args_c = match args {
        Some(s) => Some(CString::new(s).context("invalid arguments (contains NULL)")?),
        None => None,
    };

    clean_environment();

    let ret = unsafe {
        // safety: pointers must not outlive CString objects, don't move out or drop yet
        let file_p = file_c.as_ptr();
        let args_p = args_c.as_ref().map(|cs| cs.as_ptr()).unwrap_or(ptr::null());

        let ret = ShellExecuteA(
            ptr::null_mut(), // hwnd
            ptr::null(),     // lpOperation
            file_p,          // lpFile
            args_p,          // lpParameters
            ptr::null(),     // lpDirectory
            SW_SHOWNORMAL,   // nShowCmd
        );

        // no-op, but won't compile if file_c or args_c got moved/dropped
        #[cfg(debug_assertions)]
        let (_, _) = (file_c, args_c);

        // ShellExecuteA return an integer typed as HINSTANCE (for compatibility, of course)
        ret as u32
    };

    check_shellexecute_status(ret)?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
