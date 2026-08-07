; System Monitor installer - Inno Setup 6
#ifndef AppVersion
  #define AppVersion "2.3.0"
#endif
#define MyAppName "System Monitor"
#define MyAppPublisher "Xenonesis"
#define MyAppExeName "system-monitor.exe"
#define MyAppId "{{3F2A9C41-8E7D-4B6A-9C21-5D8E4F1A7B62}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL=https://github.com/Xenonesis/sysmon
DefaultDirName={autopf}\System Monitor
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir=downloads
OutputBaseFilename=SystemMonitor-{#AppVersion}-setup
SetupIconFile=assets\icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
ArchitecturesInstallIn64BitMode=x64os

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "target\release\system-monitor.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
