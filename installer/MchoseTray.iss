; Build the Rust release binary first: cargo build --release
; Then compile this file with Inno Setup 7.

#define MyAppName "MCHOSE Tray"
#define MyAppVersion GetVersionNumbersString("..\\target\\release\\mchose-tray.exe")
#define MyAppPublisher "mitsimi"
#define MyAppURL "https://github.com/mitsimi/mchose-tray"
#define MyAppExeName "mchose-tray.exe"

[Setup]
AppId={{B66C359E-A5B2-4B5B-913E-EBBA470EA8A2}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=MCHOSE-Tray-Setup-{#MyAppVersion}
SetupIconFile=..\assets\mchose.ico
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "startup"; Description: "Start MCHOSE Tray when I sign in"; Flags: checkedonce

[Files]
Source: "..\target\release\mchose-tray.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\MCHOSE Tray"; Filename: "{app}\{#MyAppExeName}"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "MCHOSE Tray"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch MCHOSE Tray"; Flags: nowait postinstall skipifsilent
