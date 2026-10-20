param(
    [ValidateSet('cuda', 'cpu', 'directml')]
    [string]$Transcription = 'cuda',
    # The built-in model for fillers and retakes (llama.cpp on Vulkan). Building it needs the
    # Vulkan SDK and LLVM (for libclang); -NoLocalLlm leaves it out.
    [switch]$NoLocalLlm
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$transcriptionFeature = switch ($Transcription) {
    'cuda' { 'parakeet-cuda' }
    'cpu' { 'parakeet' }
    'directml' { 'parakeet-directml' }
}
$features = "tauri-app,$transcriptionFeature"
if (-not $NoLocalLlm) {
    $features += ',local-llm'
    if (-not $env:VULKAN_SDK) {
        $sdk = Get-ChildItem 'C:\VulkanSDK' -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
        if (-not $sdk) { throw 'The local model needs the Vulkan SDK (winget install KhronosGroup.VulkanSDK), or pass -NoLocalLlm' }
        $env:VULKAN_SDK = $sdk.FullName
    }
    if (-not $env:LIBCLANG_PATH) { $env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin' }
    # llama.cpp's nested CMake build for its Vulkan shader generator passes Windows' 260
    # character path limit under src-tauri\target (MSVC error C1041), and the Visual Studio
    # generator installs that tool from the wrong configuration. Build in a short folder with
    # Ninja, in the Visual Studio developer environment.
    if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:SystemDrive 'aeb' }
    $env:CMAKE_GENERATOR = 'Ninja'
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $vs) { throw 'The local model needs the Visual Studio C++ build tools' }
    $vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
    cmd /c "`"$vcvars`" >nul && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') { Set-Item -Path "env:$($Matches[1])" -Value $Matches[2] }
    }
}
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $repoRoot 'src-tauri\target' }
# The CUDA build ships ONNX Runtime's CUDA provider beside the app; CUDA and cuDNN themselves
# are the GPU pack users download from transcription settings. The provider DLLs sit in the
# target folder, which can be anywhere, so the bundle config naming them is made here.
$bundleConfig = 'tauri.ffmpeg.conf.json'
$providers = Join-Path $targetDir 'ort-providers'

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
    if ($Transcription -eq 'cuda') {
        # ONNX Runtime's build drops its provider DLLs in the target folder. Tauri copies
        # resources into that same folder, so they are bundled from a copy of their own.
        Invoke-Checked 'cargo' @('build', '--release', '--lib', '--features', $features)
        New-Item -ItemType Directory -Force $providers | Out-Null
        $config = Get-Content -Raw 'tauri.ffmpeg.conf.json' | ConvertFrom-Json
        foreach ($dll in 'onnxruntime_providers_shared.dll', 'onnxruntime_providers_cuda.dll') {
            Copy-Item -Force (Join-Path $targetDir "release\$dll") (Join-Path $providers $dll)
            $config.bundle.resources | Add-Member -NotePropertyName (Join-Path $providers $dll) -NotePropertyValue $dll
        }
        $bundleConfig = Join-Path $targetDir 'tauri.cuda.conf.json'
        $config | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $bundleConfig
    }
    Invoke-Checked 'cargo' @('tauri', 'build', '--ci', '--features', $features, '--bundles', 'nsis', '--config', $bundleConfig)
    Write-Output "Installer output: $targetDir\release\bundle\nsis"
} finally {
    Pop-Location
}
