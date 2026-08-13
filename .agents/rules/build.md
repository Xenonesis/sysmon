# Sysmon Build Rules

- **Always build installable setup.exe** using Inno Setup — never a portable bare .exe.
- When the user asks for " latest app\, \build karo\, or any deliverable for the sysmon project:
 1. Run cargo build --release to compile the binary.
 2. Run & 'C:\Program Files (x86)\Inno Setup 6\iscc.exe' /DAppVersion=<version> installer.iss from the sysmon root to produce downloads\SystemMonitor-<version>-setup.exe.
 3. Provide the *-setup.exe file to the user — **never** the raw system-monitor.exe.
- The portable system-monitor.exe in arget\release is an internal build artifact only. Do not share it with the user.
