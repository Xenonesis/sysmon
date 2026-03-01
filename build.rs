use std::env;
use std::path::PathBuf;

fn main() {
    if cfg!(target_os = "windows") {
        let out_dir = env::var("OUT_DIR").unwrap();
        let target_icon = PathBuf::from(&out_dir).join("icon.ico");

        let img = image::open("assets/icon.png").expect("Failed to open icon.png");
        img.save_with_format(&target_icon, image::ImageFormat::Ico)
            .expect("Failed to save icon.ico");

        let mut res = winres::WindowsResource::new();

        // ── Application Icon ──
        res.set_icon(target_icon.to_str().unwrap());

        // ── Enterprise Version Information ──
        // These fields appear in File Properties → Details tab
        let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "1.0.0".to_string());
        let version_parts: Vec<&str> = version.split('.').collect();
        let major = version_parts.first().unwrap_or(&"1");
        let minor = version_parts.get(1).unwrap_or(&"0");
        let patch = version_parts.get(2).unwrap_or(&"0");

        res.set("CompanyName", "Xenonesis");
        res.set("FileDescription", "System Monitor — Enterprise System Monitoring Suite");
        res.set("ProductName", "System Monitor");
        res.set("OriginalFilename", "system-monitor.exe");
        res.set("InternalName", "system-monitor");
        res.set("LegalCopyright", "Copyright © 2024-2026 Xenonesis. All rights reserved.");
        res.set("FileVersion", &format!("{}.{}.{}.0", major, minor, patch));
        res.set("ProductVersion", &format!("{}.{}.{}.0", major, minor, patch));

        // ── UAC Manifest — Request Administrator Elevation ──
        // The app needs elevated privileges for:
        //   • RAM cleaning (EmptyWorkingSet on other processes)
        //   • Process priority changes
        //   • Startup program management (HKLM registry)
        //   • Process suspension (NtSuspendProcess)
        res.set_manifest(r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="Xenonesis.SystemMonitor"
    type="win32"
  />
  <description>System Monitor — Enterprise System Monitoring Suite</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 / 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" />
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}" />
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#);

        res.compile().expect("Failed to compile Windows resources");
    }
}
