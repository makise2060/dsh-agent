; DSH Agent Inno Setup Script
; 参考 Bettbox 的安装体验：现代界面 + 中英双语 + 进程检测

#ifndef APP_VERSION
  #define APP_VERSION "1.0.0"
#endif

[Setup]
AppId={{com.dsh-agent.app}}
AppVersion={#APP_VERSION}
AppName=DSH Agent
AppPublisher=DrPepper
AppPublisherURL=https://github.com/makise2060/dsh-agent
AppSupportURL=https://github.com/makise2060/dsh-agent/issues
AppUpdatesURL=https://github.com/makise2060/dsh-agent/releases
; 安装到 Program Files，需要管理员权限
DefaultDirName={autopf}\DSH Agent
DefaultGroupName=DSH Agent
DisableProgramGroupPage=yes
OutputDir=.
OutputBaseFilename=DSH Agent_{#APP_VERSION}_x64-setup
Compression=lzma2
SolidCompression=yes
SetupIconFile=icons\icon.ico
UninstallDisplayIcon={app}\dsh-agent.exe
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
CloseApplicationsFilter=dsh-agent.exe
SetupLogging=yes
; 不显示选择目标位置的页面（简化安装流程）
DisableDirPage=no
; 不显示准备安装页面
DisableReadyPage=yes

[Languages]
Name: "chinesesimplified"; MessagesFile: "Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
chinesesimplified.ShuttingDownApp=正在关闭运行中的 DSH Agent...
chinesesimplified.RemoveUserDataPrompt=是否要同时删除用户数据（配置文件、会话记录等）？%n%n此操作无法撤销。
english.ShuttingDownApp=Shutting down running DSH Agent...
english.RemoveUserDataPrompt=Do you want to remove all user data (config files, sessions, etc.)?%n%nThis action cannot be undone.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce

[Files]
; 主程序
Source: "target\release\dsh-agent.exe"; DestDir: "{app}"; Flags: ignoreversion
; WebView2 Loader DLL（Tauri 运行时必需，可能不存在则跳过）
Source: "target\release\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{autoprograms}\DSH Agent"; Filename: "{app}\dsh-agent.exe"
Name: "{autodesktop}\DSH Agent"; Filename: "{app}\dsh-agent.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\dsh-agent.exe"; Description: "{cm:LaunchProgram,DSH Agent}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ShouldCleanUserData: Boolean;

function IsProcessRunning(ProcessName: String): Boolean;
var
  ResultCode: Integer;
begin
  Exec('cmd.exe', '/c tasklist /fi "imagename eq ' + ProcessName + '" 2>nul | find /i "' + ProcessName + '" >nul', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := (ResultCode = 0);
end;

procedure ForceKillProcesses;
var
  ResultCode: Integer;
  WaitCount: Integer;
begin
  if IsProcessRunning('dsh-agent.exe') then
  begin
    // 先尝试优雅关闭（按进程树）
    Exec('taskkill', '/im dsh-agent.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    // 等待最多 3 秒
    WaitCount := 0;
    while (WaitCount < 6) and IsProcessRunning('dsh-agent.exe') do
    begin
      Sleep(500);
      WaitCount := WaitCount + 1;
    end;
    
    // 强制结束
    if IsProcessRunning('dsh-agent.exe') then
      Exec('taskkill', '/f /im dsh-agent.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
  
  // dsh.exe 和 node.exe 是 dsh-agent 的子进程，
  // 上面的 /T 已经按进程树杀掉了，不需要单独处理
end;

function InitializeSetup(): Boolean;
begin
  // 安装前先关闭正在运行的实例
  if IsProcessRunning('dsh-agent.exe') then
  begin
    ForceKillProcesses;
  end;
  Result := True;
end;

function InitializeUninstall(): Boolean;
var
  Response: Integer;
begin
  Response := MsgBox(CustomMessage('RemoveUserDataPrompt'), mbConfirmation, MB_YESNOCANCEL);
  
  if Response = IDCANCEL then
  begin
    Result := False;
  end
  else
  begin
    ShouldCleanUserData := (Response = IDYES);
    Result := True;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
  begin
    ForceKillProcesses;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    ForceKillProcesses;
  end;
  
  if CurUninstallStep = usPostUninstall then
  begin
    if ShouldCleanUserData then
    begin
      // 清理 DSH_HOME
      DelTree(ExpandConstant('{userprofile}\.dsh'), True, True, True);
      // 清理 AppData
      DelTree(ExpandConstant('{userappdata}\com.dsh-agent.app'), True, True, True);
    end;
  end;
end;
