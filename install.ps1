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
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $SourceExe -UseBasicParsing
    } catch {
        Write-Warning "[uid-agent] Failed to download precompiled Windows binary from GitHub (HTTP 404/Connection Error)."
        Write-Warning "To install without compiling, please compile 'uid-agent.exe' and commit/push it to the root of the 'uid-agent' GitHub repository."
        Write-Error "Please compile the project first on Windows using 'cargo build --release' and run '.\install.ps1' locally inside the cloned directory."
        exit 1
    }
}

# Check if pkcs11-tool is present or OpenSC is installed
$HasPkcs11 = $false
if (Get-Command "pkcs11-tool" -ErrorAction SilentlyContinue) {
    $HasPkcs11 = $true
} elseif (Test-Path "C:\Program Files\OpenSC Project\OpenSC\bin\pkcs11-tool.exe" -ErrorAction SilentlyContinue) {
    $HasPkcs11 = $true
} elseif (Test-Path "C:\Program Files (x86)\OpenSC Project\OpenSC\bin\pkcs11-tool.exe" -ErrorAction SilentlyContinue) {
    $HasPkcs11 = $true
}

if (-not $HasPkcs11) {
    Write-Host "[uid-agent] OpenSC / pkcs11-tool is required for USB Token signing but not found."
    Write-Host "[uid-agent] Downloading and launching OpenSC installation wizard..."
    
    $MsiPath = Join-Path $env:TEMP "OpenSC-0.25.1_win64.msi"
    $MsiUri = "https://github.com/OpenSC/OpenSC/releases/download/0.25.1/OpenSC-0.25.1_win64.msi"
    try {
        Invoke-WebRequest -Uri $MsiUri -OutFile $MsiPath -UseBasicParsing
        Write-Host "[uid-agent] Running OpenSC installer (please click 'Yes' on the Windows permission prompt)..."
        $InstallProcess = Start-Process msiexec.exe -ArgumentList "/i `"$MsiPath`"" -Wait -PassThru
        if ($InstallProcess.ExitCode -eq 0) {
            Write-Host "[uid-agent] OpenSC installed successfully."
        } else {
            Write-Warning "[uid-agent] OpenSC installation finished with exit code $($InstallProcess.ExitCode)."
            Write-Warning "You may need to download and install OpenSC manually from: https://github.com/OpenSC/OpenSC/releases"
        }
    } catch {
        Write-Warning "[uid-agent] Failed to download or install OpenSC: $_"
        Write-Warning "Please download and install OpenSC manually from: https://github.com/OpenSC/OpenSC/releases"
    }
}

# Stop any running instances of uid-agent before copying to prevent file locking
Write-Host "[uid-agent] Stopping running instances of uid-agent..."
Get-Process -Name "uid-agent" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

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
