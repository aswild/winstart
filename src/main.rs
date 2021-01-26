use std::env;
use std::ffi::CString;
use std::fmt::Write;
use std::ptr;

use anyhow::{anyhow, Context, Result};

mod bindings {
    ::windows::include_bindings!();
}
use bindings::windows::win32::{shell::ShellExecuteA, system_services::SW_SHOWNORMAL};

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
            Default::default(), // hwnd, default is NULL
            ptr::null(),        // lpOperation
            file_p,             // lpFile
            args_p,             // lpParameters
            ptr::null(),        // lpDirectory
            SW_SHOWNORMAL,      // nShowCmd
        );

        // no-op, but won't compile if file_c or args_c got moved/dropped
        #[cfg(debug_assertions)]
        let (_, _) = (file_c, args_c);

        ret
    };

    eprintln!("ShellExecute returned {:?}", ret);

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
