param(
    [Parameter(Mandatory)][string]$BaselineInstaller,
    [Parameter(Mandatory)][string]$UpdateInstaller,
    [Parameter(Mandatory)][string]$ExpectedVersion
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'Run this installer test only on an isolated GitHub-hosted Windows runner.'
}
$installedDirectory = Join-Path $env:LOCALAPPDATA 'Aywa'
$installedExecutable = Join-Path $installedDirectory 'Aywa.exe'
$dataDirectory = Join-Path $env:LOCALAPPDATA 'com.aywa.speech'
if ((Test-Path -LiteralPath $installedDirectory) -or (Test-Path -LiteralPath $dataDirectory)) {
    throw 'The runner already contains an Aywa installation or data. Refusing to overwrite it.'
}
foreach ($installerPath in @($BaselineInstaller, $UpdateInstaller)) {
    if (!(Test-Path -LiteralPath $installerPath -PathType Leaf)) { throw "Missing installer: $installerPath" }
}

function Invoke-TestInstaller([string]$Path, [string[]]$Arguments) {
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (!$process.WaitForExit(120000)) {
        taskkill.exe /PID $process.Id /T /F | Out-Null
        throw 'Installer did not finish within two minutes.'
    }
    if ($process.ExitCode -ne 0) { throw "Installer failed with exit code $($process.ExitCode)." }
}

function Get-InstalledVersion {
    $info = (Get-Item -LiteralPath $installedExecutable).VersionInfo
    # Tauri overrides numeric Windows versions; display strings still use Cargo's version.
    return [version]::new($info.ProductMajorPart, $info.ProductMinorPart, $info.ProductBuildPart)
}

try {
    Invoke-TestInstaller $BaselineInstaller @('/S')
    if (!(Test-Path -LiteralPath $installedExecutable)) { throw 'Baseline installation failed.' }
    $baselineVersion = Get-InstalledVersion
    if ($baselineVersion -ne [version]'0.0.0') {
        throw "Expected baseline executable version 0.0.0, found $baselineVersion."
    }
    $fixtures = @{
        'state/settings.json' = '{"version":1,"saveHistory":true,"hotkey":"control+alt+Space","rules":[{"heardAs":"test","useInstead":"passed"}]}'
        'history/index.json' = '{"version":1,"entries":[{"id":"transcript-update-test","createdAtEpochMs":1,"durationMs":100,"language":null,"preview":"Saved transcript","characterCount":16}]}'
        'history/transcripts/transcript-update-test.txt' = 'Saved transcript'
        'models/update-test/model.nemo' = 'Model preservation sentinel'
        'runtime/update-preservation.txt' = 'Runtime preservation sentinel'
    }
    $hashes = @{}
    foreach ($relativePath in $fixtures.Keys) {
        $fixturePath = Join-Path $dataDirectory $relativePath
        New-Item -ItemType Directory -Path (Split-Path -Parent $fixturePath) -Force | Out-Null
        [System.IO.File]::WriteAllText($fixturePath, $fixtures[$relativePath])
        $hashes[$relativePath] = (Get-FileHash -LiteralPath $fixturePath -Algorithm SHA256).Hash
    }

    Invoke-TestInstaller $UpdateInstaller @('/S', '/UPDATE', '/R')
    $actualVersion = Get-InstalledVersion
    if ($actualVersion -ne [version]$ExpectedVersion) { throw "Expected installed version $ExpectedVersion, found $actualVersion." }
    $deadline = (Get-Date).AddSeconds(30)
    do {
        $restarted = @(Get-Process -Name Aywa -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installedExecutable })
        if ($restarted.Count) { break }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    if (!$restarted.Count) { throw 'The updated app did not restart.' }
    foreach ($relativePath in $fixtures.Keys) {
        if ((Get-FileHash -LiteralPath (Join-Path $dataDirectory $relativePath) -Algorithm SHA256).Hash -ne $hashes[$relativePath]) {
            throw "Update changed preserved data: $relativePath"
        }
    }
    Write-Output 'Windows upgrade, restart, and local-data preservation passed.'
} finally {
    Get-Process -Name Aywa -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installedExecutable } | ForEach-Object {
        # The runner is isolated, and the executable path identifies only this test's process tree.
        taskkill.exe /PID $_.Id /T /F | Out-Null
    }
    $uninstallerPath = Join-Path $installedDirectory 'uninstall.exe'
    # _?= must be last and unquoted. It prevents NSIS from relaunching in TEMP.
    if (Test-Path -LiteralPath $uninstallerPath) { Invoke-TestInstaller $uninstallerPath @('/S', "_?=$installedDirectory") }
    $resolvedInstalledDirectory = [System.IO.Path]::GetFullPath($installedDirectory)
    $expectedInstalledDirectory = [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Aywa'))
    if ($resolvedInstalledDirectory -ne $expectedInstalledDirectory) { throw 'Unexpected test installation directory.' }
    if (Test-Path -LiteralPath $resolvedInstalledDirectory) { Remove-Item -LiteralPath $resolvedInstalledDirectory -Recurse -Force }
    $resolvedDataDirectory = [System.IO.Path]::GetFullPath($dataDirectory)
    $expectedDataDirectory = [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'com.aywa.speech'))
    if ($resolvedDataDirectory -ne $expectedDataDirectory) { throw 'Unexpected test data directory.' }
    if (Test-Path -LiteralPath $resolvedDataDirectory) { Remove-Item -LiteralPath $resolvedDataDirectory -Recurse -Force }
}
