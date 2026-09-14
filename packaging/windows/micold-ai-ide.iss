; Micold AI IDE: per-user Windows installer (feature 030).
; Normative values: specs/030-windows-installer/contracts/windows-installer.md.
; Built by scripts/windows-installer.sh, which passes /DAppVersion, /DArch and /DBinDir.

#ifndef AppVersion
#error AppVersion is not defined: build with scripts/windows-installer.sh
#endif
#ifndef Arch
#error Arch is not defined: build with scripts/windows-installer.sh
#endif
#ifndef BinDir
#error BinDir is not defined: build with scripts/windows-installer.sh
#endif

[Setup]
; Pinned forever: Windows keys the uninstall entry on it, and a new value would install a second
; copy instead of upgrading (FR-006, FR-008). `{{` is an escaped `{`.
AppId={{1B19A6AC-4C91-4033-88EA-F7F283127C8A}
AppName=Micold AI IDE
; Injected from [workspace.package] in Cargo.toml; never typed here (FR-013).
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersion}
AppPublisher=Micold AI IDE contributors
; {autopf} resolves to %LOCALAPPDATA%\Programs under lowest privileges (FR-004).
DefaultDirName={autopf}\Micold AI IDE
DefaultGroupName=Micold AI IDE
DisableProgramGroupPage=yes
; Paths relative to this script's directory.
SetupIconFile=..\..\assets\icon\icon.ico
UninstallDisplayIcon={app}\micold-ai-ide.exe
LicenseFile=..\..\LICENSE
WizardStyle=modern
; The release asset name; site/stage.sh and docs/user-guide/install-windows.md link it (FR-001).
OutputBaseFilename=micold-ai-ide-{#AppVersion}-{#Arch}-setup
; Per user: installs under %LOCALAPPDATA%\Programs with no UAC prompt, and never offers an
; all-users install (FR-004). PrivilegesRequiredOverridesAllowed stays absent for that reason.
PrivilegesRequired=lowest
; The app window holds this mutex (micold_core::process::APP_MUTEX_NAME); setup asks the user to
; close it first (FR-009). The daemon does not hold it, since no one can close a windowless process:
; [Code] StopDaemon stops it.
AppMutex=Local\MicoldAIIDE
; When the user continues, Restart Manager closes a still-open app window (FR-009).
CloseApplications=force
; Never relaunch the app setup closed; the installer starts nothing on its own (FR-011).
RestartApplications=no
; One package per architecture; each refuses the other with Inno's own message (FR-015).
#if Arch == "x64"
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible and not arm64
#elif Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
#error Arch must be x64 or arm64
#endif

[Files]
; Exactly the app and the daemon it spawns (FR-002, FR-012). Never a wildcard: the showcase is built
; into the same directory. Guarded by crates/micold-client/tests/packaging_excludes_showcase.rs.
Source: "{#BinDir}\micold-ai-ide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\micold-daemon.exe"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Icons]
Name: "{autoprograms}\Micold AI IDE"; Filename: "{app}\micold-ai-ide.exe"
Name: "{autodesktop}\Micold AI IDE"; Filename: "{app}\micold-ai-ide.exe"; Tasks: desktopicon

[Run]
; Unchecked by default: the installer never starts the app unless the user asks (FR-011).
Filename: "{app}\micold-ai-ide.exe"; Description: "Launch Micold AI IDE"; Flags: postinstall nowait skipifsilent unchecked

[UninstallDelete]
; Only the daemon's runtime dir (pid record, socket). Settings and session data under
; {userappdata} and {localappdata}\micold-ai-ide\data are the user's and stay (FR-007).
Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run"

[Code]
// Restart Manager closes the app window, but not the windowless daemon, and a live daemon keeps its
// exe locked. So install and uninstall stop it first (research R12, FR-009, FR-023). Guarded by
// crates/micold-core/tests/windows_installer_in_use.rs; exercised by scripts/windows-install-smoke.sh.

const
  DaemonStillRunning = 'Micold AI IDE''s session service is still running. Close the application and choose Retry.';

// Stops this user's daemon. Returns '' once it is gone, or the message to show when it is not.
function StopDaemon(): String;
var
  PidText: AnsiString;
  Pid: String;
  ResultCode: Integer;
begin
  Result := '';
  exit; // MUTANT (A9): the daemon is never stopped
  if LoadStringFromFile(ExpandConstant('{localappdata}\micold-ai-ide\run\micold-daemon.pid'), PidText) then
    Pid := Trim(String(PidText));
  if StrToIntDef(Pid, 0) > 0 then
  begin
    // Only a process whose image is micold-daemon.exe: a stale record can name a reused pid.
    if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      '-NoProfile -NonInteractive -Command "$p = Get-Process -Id ' + Pid + ' -ErrorAction SilentlyContinue; ' +
      'if ($p -and $p.Path -like ''*\micold-daemon.exe'') { Stop-Process -Id ' + Pid + ' -Force; ' +
      'if (-not $p.WaitForExit(5000)) { exit 1 } }; exit 0"',
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      ResultCode := 1;
    if ResultCode <> 0 then
      Result := DaemonStillRunning;
  end
  else
    // No record: stop any daemon of this user's, wherever it was installed.
    Exec(ExpandConstant('{sys}\taskkill.exe'),
      '/F /IM micold-daemon.exe /FI "USERNAME eq ' + GetUserNameString() + '"',
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := StopDaemon();
end;

function InitializeUninstall(): Boolean;
var
  Error: String;
begin
  Error := StopDaemon();
  if (Error <> '') and not UninstallSilent() then
    MsgBox(Error, mbError, MB_OK);
  // A silent uninstall has no one to retry, so it goes ahead.
  Result := (Error = '') or UninstallSilent();
end;

// The ready page lists what setup will do; stopping the daemon ends the user's sessions, so it says
// so before they confirm (FR-009).
function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo, MemoTypeInfo,
  MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
begin
  Result := '';
  if MemoUserInfoInfo <> '' then Result := Result + MemoUserInfoInfo + NewLine + NewLine;
  if MemoDirInfo <> '' then Result := Result + MemoDirInfo + NewLine + NewLine;
  if MemoTypeInfo <> '' then Result := Result + MemoTypeInfo + NewLine + NewLine;
  if MemoComponentsInfo <> '' then Result := Result + MemoComponentsInfo + NewLine + NewLine;
  if MemoGroupInfo <> '' then Result := Result + MemoGroupInfo + NewLine + NewLine;
  if MemoTasksInfo <> '' then Result := Result + MemoTasksInfo + NewLine + NewLine;
  Result := Result + 'Running sessions will be stopped.';
end;
