[CmdletBinding()]
param(
    [ValidateSet('portable', 'installer')]
    [string]$Target = 'portable'
)

$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$userProfilePath = [Environment]::GetFolderPath('UserProfile')
$separator = [char]0x1f
$privacyFlags = @(
    "--remap-path-prefix=$projectRoot=<workspace>"
)

if ($userProfilePath) {
    $privacyFlags += "--remap-path-prefix=$userProfilePath=<user-home>"
}

$previousEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
$encodedFlags = @()
if ($previousEncodedFlags) {
    $encodedFlags += $previousEncodedFlags
}
$encodedFlags += $privacyFlags

$env:CARGO_ENCODED_RUSTFLAGS = $encodedFlags -join $separator

Push-Location $projectRoot
try {
    $tauriArgs = @('@tauri-apps/cli', 'build')
    if ($Target -eq 'portable') {
        $tauriArgs += @('--bundles', 'none')
    }

    & npx.cmd @tauriArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri $Target build failed with exit code $LASTEXITCODE"
    }

    if ($Target -eq 'portable') {
        & node scripts/copy-portable-release.js
        if ($LASTEXITCODE -ne 0) {
            throw "Portable release copy failed with exit code $LASTEXITCODE"
        }
    }
}
finally {
    Pop-Location
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedFlags
}
