[Setup]
AppName=System Monitor
AppVersion=2.2.0
AppPublisher=System Monitor Contributors
AppPublisherURL=https://github.com/Xenonesis/sysmon
DefaultDirName={autopf}\System Monitor
DefaultGroupName=System Monitor
OutputDir=downloads
OutputBaseFilename=SystemMonitor-Setup
Compression=lzma2/max
SolidCompression=yes
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\system-monitor.exe
ArchitecturesInstallIn64BitMode=x64
DisableWelcomePage=no
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "target\release\system-monitor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\System Monitor"; Filename: "{app}\system-monitor.exe"
Name: "{group}\{cm:UninstallProgram,System Monitor}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\System Monitor"; Filename: "{app}\system-monitor.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\system-monitor.exe"; Description: "{cm:LaunchProgram,System Monitor}"; Flags: nowait postinstall skipifsilent
