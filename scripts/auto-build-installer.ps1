# Builds the Windows x64 installer (NSIS setup .exe only) in the background after a Claude Code
# task finishes, so the latest change is ready to install and test. Run by the Stop hook in
# .claude/settings.json; returns at once.
#
# - Builds only when front-end/ or src-tauri/ changed since the last build (committed or not).
# - One build at a time: a change made while one runs is built right after it.
# - The installer lands in src-tauri/target/release/bundle/nsis and is copied to
#   output/installer/AeroEdits_x64-setup.exe; the log is output/installer/build.log.
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

# What the app is built from: the commit plus every change and new file under the app's folders.
function Get-Fingerprint {
    Push-Location -LiteralPath $repo
    try {
        $paths = @('front-end', 'src-tauri')
        $parts = @(
            (git rev-parse HEAD),
            (git diff HEAD -- @paths | Out-String),
            (git ls-files -o --exclude-standard -- @paths | git hash-object --stdin-paths | Out-String)
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

if (-not $Build) {
    $fingerprint = Get-Fingerprint
    if ((Test-Path $last) -and (Get-Content $last) -eq $fingerprint) { exit 0 }
    if (Test-Running) {
        New-Item -ItemType File -Force -Path $pending | Out-Null
        exit 0
    }
    $process = Start-Process -FilePath 'pwsh' -WindowStyle Hidden -PassThru `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath, '-Build')
    Set-Content -Path $pidFile -Value $process.Id
    exit 0
}

# The build itself, in its own hidden process.
Set-Content -Path $pidFile -Value $PID
try {
    do {
        Remove-Item $pending -ErrorAction SilentlyContinue
        $fingerprint = Get-Fingerprint
        "=== $(Get-Date -Format s) building $fingerprint" | Set-Content $log
        Push-Location -LiteralPath (Join-Path $repo 'src-tauri')
        try {
            & node scripts/fetch-ffmpeg.mjs *>> $log
            if ($LASTEXITCODE -ne 0) { throw "fetch-ffmpeg failed ($LASTEXITCODE)" }
            & cargo tauri build --ci --features 'tauri-app,parakeet-cuda' --bundles nsis --config tauri.ffmpeg.conf.json *>> $log
            if ($LASTEXITCODE -ne 0) { throw "cargo tauri build failed ($LASTEXITCODE)" }
        } finally {
            Pop-Location
        }
        $setup = Get-ChildItem (Join-Path $repo 'src-tauri\target\release\bundle\nsis\*_x64-setup.exe') |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        Copy-Item $setup.FullName (Join-Path $state 'AeroEdits_x64-setup.exe') -Force
        Set-Content -Path $last -Value $fingerprint
        "=== $(Get-Date -Format s) done: $($setup.FullName)" | Add-Content $log
    } while ((Test-Path $pending) -and (Get-Fingerprint) -ne $fingerprint)
} catch {
    "=== $(Get-Date -Format s) FAILED: $_" | Add-Content $log
} finally {
    Remove-Item $pidFile, $pending -ErrorAction SilentlyContinue
}
