fn main() {
    // If vwi.ico exists, embed it as a Windows resource so the icon
    // ships inside the EXE and shows up in Explorer, Task Manager, and
    // the system tray. No separate .ico file needed at runtime.
    if std::path::Path::new("vwi.ico").exists() {
        embed_resource::compile("vwi.rc", embed_resource::NONE);
    }
}
