$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$androidRoot = Join-Path $projectRoot 'src-tauri\gen\android'
$releaseRoot = Join-Path $projectRoot 'release'

function Resolve-AndroidSdk {
    $candidates = @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT)
    if ($env:LOCALAPPDATA) {
        $candidates += (Join-Path $env:LOCALAPPDATA 'Android\Sdk')
    }

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    throw 'Android SDK was not found. Set ANDROID_HOME or ANDROID_SDK_ROOT.'
}

function Resolve-AndroidNdk([string]$sdkRoot) {
    if ($env:NDK_HOME -and (Test-Path -LiteralPath $env:NDK_HOME)) {
        return (Resolve-Path -LiteralPath $env:NDK_HOME).Path
    }

    $ndkRoot = Join-Path $sdkRoot 'ndk'
    $latest = Get-ChildItem -LiteralPath $ndkRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if ($latest) {
        return $latest.FullName
    }

    throw "Android NDK was not found: $ndkRoot"
}

function Invoke-Native([string]$command, [string[]]$arguments) {
    & $command @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$command failed with exit code $LASTEXITCODE"
    }
}

$sdkRoot = Resolve-AndroidSdk
$ndkRoot = Resolve-AndroidNdk $sdkRoot
$env:ANDROID_HOME = $sdkRoot
$env:ANDROID_SDK_ROOT = $sdkRoot
$env:NDK_HOME = $ndkRoot

$tempRoot = if ($env:TEMP -and (Test-Path -LiteralPath $env:TEMP)) {
    (Resolve-Path -LiteralPath $env:TEMP).Path
} else {
    Join-Path $env:SystemDrive 'TaskFlowTemp'
}
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

# Keep Cargo output on an ASCII path because Android NDK/LLD and Kotlin incremental
# compilation can fail when the project path contains non-ASCII characters.
$targetRoot = if ($env:TASKFLOW_ANDROID_TARGET_DIR) {
    $env:TASKFLOW_ANDROID_TARGET_DIR
    } elseif ($tempRoot) {
        Join-Path $tempRoot 'TaskFlowAndroidTarget'
} else {
    Join-Path $env:SystemDrive 'TaskFlowAndroidTarget'
}
$env:CARGO_TARGET_DIR = $targetRoot

Push-Location $projectRoot
try {
    $tauriArgs = @('tauri', 'android', 'build', '--debug', '--target', 'aarch64', '--apk', '--ci')
        $buildLog = Join-Path $tempRoot 'taskflow-android-build.log'

    # Tauri builds Rust first and then creates a symlink into jniLibs. On Windows
    # without Developer Mode, recover from that final symlink step by copying the file.
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & npx.cmd @tauriArgs 2>&1 | Tee-Object -FilePath $buildLog
        $tauriExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorAction
    }
    $tauriOutput = if (Test-Path -LiteralPath $buildLog) { Get-Content -LiteralPath $buildLog -Raw } else { '' }

    $symlinkFailure = $tauriOutput -match 'Creation symbolic link is not allowed|symbolic link.*not allowed|Developer Mode'
    if ($tauriExitCode -ne 0 -and -not $symlinkFailure) {
        throw "Tauri Android build failed with exit code $tauriExitCode"
    }

    $rustProfile = 'debug'
    $rustLibrary = Join-Path $targetRoot "aarch64-linux-android\$rustProfile\libtaskflow_lite.so"
    if (-not (Test-Path -LiteralPath $rustLibrary)) {
        throw "Android Rust library was not found: $rustLibrary"
    }

    $jniDir = Join-Path $androidRoot 'app\src\main\jniLibs\arm64-v8a'
    New-Item -ItemType Directory -Force -Path $jniDir | Out-Null
    Copy-Item -LiteralPath $rustLibrary -Destination (Join-Path $jniDir 'libtaskflow_lite.so') -Force

    Push-Location $androidRoot
    try {
        Invoke-Native '.\gradlew.bat' @('clean', 'assembleArm64Debug', '-x', 'rustBuildArm64Debug', '-x', 'rustBuildUniversalDebug')
    }
    finally {
        Pop-Location
    }

    $apk = Join-Path $androidRoot 'app\build\outputs\apk\arm64\debug\app-arm64-debug.apk'
    if (-not (Test-Path -LiteralPath $apk)) {
        throw "Gradle finished but APK was not found: $apk"
    }

    New-Item -ItemType Directory -Force -Path $releaseRoot | Out-Null
    $apkName = (-join @([char]0x5c0f, [char]0x5149, [char]0x4efb, [char]0x52a1)) + '-android-arm64-debug.apk'
    $releaseApk = Join-Path $releaseRoot $apkName
    Copy-Item -LiteralPath $apk -Destination $releaseApk -Force
    Write-Host "Android APK generated: $releaseApk"
}
finally {
    Pop-Location
}
