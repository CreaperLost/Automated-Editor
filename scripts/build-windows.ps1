param(
    [ValidateSet('cuda', 'cpu', 'directml')]
    [string]$Transcription = 'cuda'
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$transcriptionFeature = switch ($Transcription) {
    'cuda' { 'parakeet-cuda' }
    'cpu' { 'parakeet' }
    'directml' { 'parakeet-directml' }
}

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE"
    }
}

Push-Location -LiteralPath $repoRoot
try {
    Invoke-Checked 'npm.cmd' @('--prefix', 'front-end', 'ci', '--include=dev')
    Set-Location -LiteralPath (Join-Path $repoRoot 'src-tauri')
    Invoke-Checked 'node' @('scripts/fetch-ffmpeg.mjs')
    Invoke-Checked 'cargo' @('tauri', 'build', '--ci', '--features', "tauri-app,$transcriptionFeature", '--bundles', 'nsis', '--config', 'tauri.ffmpeg.conf.json')
    Write-Output "Installer output: $repoRoot\src-tauri\target\release\bundle\nsis"
} finally {
    Pop-Location
}
