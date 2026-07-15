param(
    [string]$ManifestPath = "src-tauri/python-runtime/runtime-manifest.json",
    [string]$LockPath = "src-tauri/python-runtime/runtime-lock.json",
    [string]$OutputDir = "src-tauri/resources/python",
    [switch]$UpdateLock
)

$ErrorActionPreference = "Stop"

function Get-Sha256([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            $hash = $sha256.ComputeHash($stream)
            return ([System.BitConverter]::ToString($hash) -replace '-', '').ToLowerInvariant()
        }
        finally {
            $sha256.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Save-File([string]$Url, [string]$Path) {
    if (!(Test-Path $Path)) {
        Invoke-WebRequest -Uri $Url -OutFile $Path
    }
}

function Assert-Checksum([string]$Path, [string]$ExpectedSha256) {
    $actual = Get-Sha256 $Path
    if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
        Remove-Item $Path -Force
        throw "Checksum mismatch for $Path. Expected $ExpectedSha256, got $actual."
    }
}

$manifest = Get-Content $ManifestPath -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$pythonZipPath = Join-Path $OutputDir $manifest.pythonZip.fileName
$getPipPath = Join-Path $OutputDir $manifest.getPip.fileName

Save-File $manifest.pythonZip.url $pythonZipPath
Save-File $manifest.getPip.url $getPipPath

if ($UpdateLock) {
    $lock = [ordered]@{
        runtimeId = $manifest.runtimeId
        pythonZip = [ordered]@{
            fileName = $manifest.pythonZip.fileName
            sha256 = Get-Sha256 $pythonZipPath
        }
        getPip = [ordered]@{
            fileName = $manifest.getPip.fileName
            sha256 = Get-Sha256 $getPipPath
        }
    }
    $lockJson = $lock | ConvertTo-Json -Depth 5
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText([System.IO.Path]::GetFullPath($LockPath), $lockJson + [Environment]::NewLine, $utf8NoBom)
    Write-Host "Updated Python runtime lock file at $LockPath"
}

if (!(Test-Path $LockPath)) {
    throw "Runtime lock file missing. Run npm run prepare:python-runtime:lock once and commit $LockPath."
}

$locked = Get-Content $LockPath -Raw | ConvertFrom-Json
Assert-Checksum $pythonZipPath $locked.pythonZip.sha256
Assert-Checksum $getPipPath $locked.getPip.sha256

Write-Host "Prepared Python runtime resources in $OutputDir"
