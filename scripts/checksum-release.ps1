$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$configPath = Join-Path $projectRoot "src-tauri\tauri.conf.json"
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$fileName = "SeekDash_Setup_$($config.version)_x64.exe"
$installerPath = Join-Path $projectRoot "release\$fileName"

if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
    throw "Installer not found: $installerPath"
}

$stream = [System.IO.File]::OpenRead($installerPath)
try {
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $algorithm.ComputeHash($stream)
    }
    finally {
        $algorithm.Dispose()
    }
}
finally {
    $stream.Dispose()
}
$hash = ([System.BitConverter]::ToString($hashBytes)).Replace("-", "").ToLowerInvariant()
$checksumPath = "$installerPath.sha256"
Set-Content -LiteralPath $checksumPath -Value "$hash  $fileName" -Encoding ascii

Write-Output "SHA-256: $hash"
Write-Output "Written: $checksumPath"
