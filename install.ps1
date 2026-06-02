# PowerShell script to install uid-agent on Windows

$ErrorActionPreference = "Stop"

Write-Host "[uid-agent] Starting installation on Windows..."

# Target folder for binary
$InstallDir = Join-Path $env:LOCALAPPDATA "uid-agent"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$SourceExe = ""
if ($MyInvocation.MyCommand.Path) {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    if ($ScriptDir) {
        $SourceExe = Join-Path $ScriptDir "uid-agent.exe"
    }
}

if (-not $SourceExe) {
    $SourceExe = "uid-agent.exe"
}

if (-not (Test-Path $SourceExe -ErrorAction SilentlyContinue)) {
    $SourceExe = "target\release\uid-agent.exe"
}

if (-not (Test-Path $SourceExe -ErrorAction SilentlyContinue)) {
    Write-Host "[uid-agent] Local executable not found, downloading precompiled binary..."
    $SourceExe = Join-Path $env:TEMP "uid-agent.exe"
    $Uri = "https://raw.githubusercontent.com/oneuid/uid-agent/main/uid-agent.exe"
    Invoke-WebRequest -Uri $Uri -OutFile $SourceExe -UseBasicParsing
}

# Copy binary to LocalAppData
$DestExe = Join-Path $InstallDir "uid-agent.exe"
Copy-Item -Path $SourceExe -Destination $DestExe -Force
Write-Host "[uid-agent] Copied executable to $DestExe"

# Register to run at startup via HKCU Run registry key
$RegistryPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$RegistryValueName = "UIDAgent"
$RegistryValueData = "`"$DestExe`" daemon"

Set-ItemProperty -Path $RegistryPath -Name $RegistryValueName -Value $RegistryValueData
Write-Host "[uid-agent] Registered to run on startup in Current User Registry."

# Run the agent in background immediately
Start-Process -FilePath $DestExe -ArgumentList "daemon" -WindowStyle Hidden
Write-Host "[uid-agent] Started uid-agent background daemon."
Write-Host "[uid-agent] Installation completed successfully."
