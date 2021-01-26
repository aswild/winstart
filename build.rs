fn main() {
    windows::build!(
        windows::win32::shell::ShellExecuteA
        windows::win32::system_services::SW_SHOWNORMAL
    );
}
