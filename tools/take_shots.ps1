param(
    [string]$Lang = "zh",
    [string]$DemoDir = "D:\zsm-demo\zcode",
    [int]$LaunchMs = 3500,
    [int]$PrivacySettleMs = 4500
)
# Regenerate README screenshots for one language:
#   <lang>-dark/light/hc.png  (main window, sessions tab, three themes)
#   <lang>-privacy.png        (privacy tab, dark theme)
# Writes zsm settings.json pointing at FICTIONAL demo data and ALWAYS
# restores the original settings afterwards. Settings are written WITHOUT
# a BOM — a BOM would make serde_json reject the file and the app would
# silently fall back to the REAL data directory.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root "target\release\zsm-gui.exe"
if (!(Test-Path $exe)) { throw "release exe not built: $exe" }
$shots = Join-Path $root "target\shots"
New-Item -ItemType Directory -Force $shots | Out-Null
$settings = Join-Path $env:APPDATA "zsm\settings.json"
$settingsDir = Split-Path -Parent $settings
New-Item -ItemType Directory -Force $settingsDir | Out-Null
$hadSettings = Test-Path $settings
$backup = if ($hadSettings) { [System.IO.File]::ReadAllText($settings) } else { $null }

function Write-ZsmSettings($obj) {
    $json = ConvertTo-Json $obj
    [System.IO.File]::WriteAllText($settings, $json) # UTF8, no BOM
}

try {
    python (Join-Path $root "tools\make_demo_data.py") --lang $Lang --out $DemoDir
    if ($LASTEXITCODE -ne 0) { throw "make_demo_data failed" }

    foreach ($theme in @("dark", "light", "hc")) {
        Write-ZsmSettings @{ zcodeDir = $DemoDir; language = $Lang; theme = $theme; idleMinutes = 60; pollSeconds = 4 }
        $p = Start-Process -PassThru $exe
        Start-Sleep -Milliseconds $LaunchMs
        & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root "tools\screenshot.ps1") -ProcId $p.Id -out (Join-Path $shots "$Lang-$theme.png")
        Stop-Process -Id $p.Id -Force
        Start-Sleep -Milliseconds 700
    }

    # privacy tab (second tab) on the dark theme
    Write-ZsmSettings @{ zcodeDir = $DemoDir; language = $Lang; theme = "dark"; idleMinutes = 60; pollSeconds = 4 }
    $p = Start-Process -PassThru $exe
    Start-Sleep -Milliseconds $LaunchMs
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root "tools\screenshot.ps1") -ProcId $p.Id -out (Join-Path $shots "$Lang-privacy.png") -Click "155,24" -SettleMs $PrivacySettleMs
    Stop-Process -Id $p.Id -Force
} finally {
    if ($hadSettings) { [System.IO.File]::WriteAllText($settings, $backup) }
    else { Remove-Item $settings -ErrorAction SilentlyContinue }
}
Write-Output "shots for '$Lang' done -> $shots"
