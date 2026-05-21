[Setup]
AppName=WinFlow
AppVersion=1.0.10
AppPublisher=Avacuoss
AppPublisherURL=https://github.com/Vacuoss
DefaultDirName={autopf}\WinFlow
DefaultGroupName=WinFlow
OutputDir=installer
OutputBaseFilename=WinFlowSetup-1.0.10
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
SetupIconFile=assets\winflow.ico
UninstallDisplayIcon={app}\winflow.exe

[Files]
Source: "dist\winflow.exe"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: "desktopicon"; Description: "Create desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Icons]
Name: "{group}\WinFlow"; Filename: "{app}\winflow.exe"
Name: "{autodesktop}\WinFlow"; Filename: "{app}\winflow.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\winflow.exe"; Description: "Launch WinFlow"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\WinFlow"