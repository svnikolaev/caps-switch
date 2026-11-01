; caps-switch — установщик Inno Setup.
; Сборка: ISCC.exe /DMyAppVersion=0.3.2 installer\caps-switch.iss
; Результат: dist\caps-switch-setup-<version>.exe
;
; В мастере — явный выбор режима автозапуска:
;   • обычный  — HKCU\...\Run, процесс стартует без повышения прав;
;   • с повышенными правами — задача Планировщика с наивысшими правами:
;     работает и в окнах, запущенных от администратора (UAC при установке).

#ifndef MyAppVersion
  #define MyAppVersion "0.3.2"
#endif

#define MyAppName "caps-switch"
#define MyAppPublisher "Sergey Nikolaev"
#define MyAppURL "https://github.com/svnikolaev/caps-switch"
#define MyAppExeName "caps-switch.exe"
#define MyAppId "{{8F3C2A54-7B1E-4C6D-9F21-4B7E5A9C1D33}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\caps-switch
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=caps-switch-setup-{#MyAppVersion}
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=yes
CloseApplicationsFilter={#MyAppExeName}

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; Взаимоисключающие режимы автозапуска (как радиокнопки).
Name: "startup_user"; Description: "{cm:StartupUser}"; GroupDescription: "{cm:StartupGroup}"; Flags: exclusive checkedonce
Name: "startup_elevated"; Description: "{cm:StartupElevated}"; GroupDescription: "{cm:StartupGroup}"; Flags: exclusive
; Дополнительно.
Name: "launch"; Description: "{cm:TaskLaunch}"; GroupDescription: "{cm:TaskGroup}"; Flags: checkedonce

[CustomMessages]
russian.StartupGroup=Режим автозапуска
russian.StartupUser=Обычный — запускать при входе, без повышения прав
russian.StartupElevated=С повышенными правами — работает и в окнах администратора (потребуется UAC)
russian.TaskGroup=Дополнительно
russian.TaskLaunch=Запустить caps-switch сейчас
russian.TaskNoAutostart=Не добавлять в автозапуск
english.StartupGroup=Autostart mode
english.StartupUser=Normal — start at logon, no elevation
english.StartupElevated=Elevated — also works in administrator windows (UAC prompt)
english.TaskGroup=Additional
english.TaskLaunch=Launch caps-switch now
english.TaskNoAutostart=Do not autostart

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"

[Registry]
; Обычный режим: автозапуск текущего пользователя.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "caps-switch"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startup_user
; Настройки приложения чистим при удалении.
Root: HKCU; Subkey: "Software\caps-switch"; Flags: uninsdeletekey

[Run]
Filename: "{app}\{#MyAppExeName}"; Flags: nowait postinstall skipifsilent; Tasks: launch

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--uninstall"; Flags: runhidden waituntilterminated; RunOnceId: "CapsSwitchUninstall"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('startup_elevated') then
  begin
    // Задача Планировщика с наивысшими правами: работает и в админских окнах.
    // Требует подтверждения UAC.
    ShellExec('runas', ExpandConstant('{app}\{#MyAppExeName}'), '--install --elevated',
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
