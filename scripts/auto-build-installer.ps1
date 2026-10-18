# Builds the Windows x64 installer (NSIS setup .exe only) in the background after a Claude Code
# task finishes, so the latest change is ready to install and test. Run by the Stop hook in
# .claude/settings.json; returns at once.
#
# - Builds only when front-end/ or src-tauri/ changed since the last build (committed or not).
# - One build at a time: a change made while one runs is built right after it.
# - Builds in folders of its own (src-tauri/target/hook, output/installer/frontend), so it never
#   blocks or overwrites a build you run yourself.
# - Says what it is doing: a line in Claude Code when a build starts, finishes or fails, and a
#   Windows notification when one finishes or fails.
# - The installer is output/installer/AeroEdits_x64-setup.exe; the log is
#   output/installer/build.log and the state output/installer/status.json.
#
# Set AEROEDITS_AUTOBUILD=0 to switch it off.
param([switch]$Build)

$ErrorActionPreference = 'Stop'
if ($env:AEROEDITS_AUTOBUILD -eq '0') { exit 0 }
$repo = Split-Path -Parent $PSScriptRoot
$state = Join-Path $repo 'output\installer'
New-Item -ItemType Directory -Force -Path $state | Out-Null
$last = Join-Path $state 'last-built.txt'
$pidFile = Join-Path $state 'build.pid'
$pending = Join-Path $state 'pending'
$log = Join-Path $state 'build.log'
$statusFile = Join-Path $state 'status.json'
$reported = Join-Path $state 'reported.txt'
$setupCopy = Join-Path $state 'AeroEdits_x64-setup.exe'
$targetDir = Join-Path $repo 'src-tauri\target\hook'
$frontendDir = Join-Path $state 'frontend'

# What the app is built from: the commit plus every change and new file under the app's folders.
function Get-Fingerprint {
    Push-Location -LiteralPath $repo
    try {
        $paths = @('front-end', 'src-tauri')
        $parts = @(
            (git rev-parse HEAD 2>$null),
            (git diff HEAD -- @paths 2>$null | Out-String),
            (git ls-files -o --exclude-standard -- @paths 2>$null | git hash-object --stdin-paths 2>$null | Out-String)
        ) -join "`n"
        $bytes = [Text.Encoding]::UTF8.GetBytes($parts)
        return [BitConverter]::ToString([Security.Cryptography.SHA256]::HashData($bytes)) -replace '-', ''
    } finally {
        Pop-Location
    }
}

function Test-Running {
    if (-not (Test-Path $pidFile)) { return $false }
    $id = Get-Content $pidFile -ErrorAction SilentlyContinue
    return [bool]($id -and (Get-Process -Id $id -ErrorAction SilentlyContinue))
}

function Get-Status {
    try { return Get-Content $statusFile -Raw | ConvertFrom-Json } catch { return $null }
}

function Set-Status([string]$state, [string]$message) {
    @{ state = $state; at = (Get-Date -Format s); message = $message } |
        ConvertTo-Json | Set-Content $statusFile
}

# A line for Claude Code (the Stop hook shows `systemMessage` to the user).
function Say([string]$message) {
    @{ systemMessage = "Installer: $message" } | ConvertTo-Json -Compress
}

function Notify([string]$title, [string]$text) {
    try {
        Add-Type -AssemblyName System.Windows.Forms, System.Drawing
        $icon = New-Object System.Windows.Forms.NotifyIcon
        $icon.Icon = [System.Drawing.SystemIcons]::Information
        $icon.Visible = $true
        $icon.ShowBalloonTip(10000, $title, $text, [System.Windows.Forms.ToolTipIcon]::Info)
        Start-Sleep -Seconds 10
        $icon.Dispose()
    } catch {
        # A notification that can't be shown is no reason to fail the build.
    }
}

if (-not $Build) {
    # Report a build that ended since the last report, once.
    $status = Get-Status
    $news = $null
    if ($status -and $status.state -ne 'running' -and (Get-Content $reported -ErrorAction SilentlyContinue) -ne $status.at) {
        Set-Content $reported $status.at
        $news = $status.message
    }
    $fingerprint = Get-Fingerprint
    if ((Test-Path $last) -and (Get-Content $last) -eq $fingerprint) {
        if ($news) { Say $news }
        exit 0
    }
    if (Test-Running) {
        New-Item -ItemType File -Force -Path $pending | Out-Null
        Say "a build is running (started $($status.at)); this change is built right after it. Log: output\installer\build.log"
        exit 0
    }
    $process = Start-Process -FilePath 'pwsh' -WindowStyle Hidden -PassThru `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath, '-Build')
    Set-Content -Path $pidFile -Value $process.Id
    $started = "building the latest change in the background (about 5-10 minutes). Log: output\installer\build.log"
    Say $(if ($news) { "$news Now $started" } else { $started })
    exit 0
}

# The build itself, in its own hidden process.
Set-Content -Path $pidFile -Value $PID
Set-Status 'running' 'building'
try {
    do {
        Remove-Item $pending -ErrorAction SilentlyContinue
        $fingerprint = Get-Fingerprint
        "=== $(Get-Date -Format s) building $fingerprint" | Set-Content $log
        # Its own front-end folder and config: the bundled FFmpeg, and the front end built into
        # output/installer/frontend instead of front-end/dist.
        $config = Get-Content (Join-Path $repo 'src-tauri\tauri.ffmpeg.conf.json') -Raw | ConvertFrom-Json -AsHashtable
        $config['build'] = @{
            beforeBuildCommand = @{
                # Unquoted: Tauri passes quotes through to Vite as part of the path.
                script = "npx tsc && npx vite build --outDir $($frontendDir -replace '\\', '/') --emptyOutDir"
                cwd = '../front-end'
            }
            frontendDist = ($frontendDir -replace '\\', '/')
        }
        $hookConfig = Join-Path $state 'tauri.hook.conf.json'
        $config | ConvertTo-Json -Depth 10 | Set-Content $hookConfig
        $env:CARGO_TARGET_DIR = $targetDir
        Push-Location -LiteralPath (Join-Path $repo 'src-tauri')
        try {
            & node scripts/fetch-ffmpeg.mjs *>> $log
            if ($LASTEXITCODE -ne 0) { throw "fetch-ffmpeg failed ($LASTEXITCODE)" }
            & cargo tauri build --ci --features 'tauri-app,parakeet-cuda' --bundles nsis --config $hookConfig *>> $log
            if ($LASTEXITCODE -ne 0) { throw "cargo tauri build failed ($LASTEXITCODE)" }
        } finally {
            Pop-Location
        }
        $setup = Get-ChildItem (Join-Path $targetDir 'release\bundle\nsis\*_x64-setup.exe') |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        Copy-Item $setup.FullName $setupCopy -Force
        Set-Content -Path $last -Value $fingerprint
        "=== $(Get-Date -Format s) done: $($setup.FullName)" | Add-Content $log
    } while ((Test-Path $pending) -and (Get-Fingerprint) -ne $fingerprint)
    $time = Get-Date -Format 'HH:mm'
    Set-Status 'done' "ready (built $time): output\installer\AeroEdits_x64-setup.exe."
    Notify 'AeroEdits installer ready' "Built $time. output\installer\AeroEdits_x64-setup.exe"
} catch {
    "=== $(Get-Date -Format s) FAILED: $_" | Add-Content $log
    $time = Get-Date -Format 'HH:mm'
    Set-Status 'failed' "the build failed at $time ($_). See output\installer\build.log."
    Notify 'AeroEdits installer build failed' "At $time. See output\installer\build.log"
} finally {
    Remove-Item $pidFile, $pending -ErrorAction SilentlyContinue
}
