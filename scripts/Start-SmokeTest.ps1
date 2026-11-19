param(
    [string]$DataRoot,
    [string]$Executable
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$testRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'src-tauri\target\smoke'))
if (-not $DataRoot) { $DataRoot = Join-Path $testRoot ([guid]::NewGuid().ToString('N')) }
$DataRoot = [IO.Path]::GetFullPath($DataRoot)
if (-not $DataRoot.StartsWith($testRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Smoke-test data must be inside src-tauri\target\smoke in this checkout.'
}
if (-not $Executable) { $Executable = Join-Path $repoRoot 'src-tauri\target\release\deckforge.exe' }
$Executable = (Resolve-Path -LiteralPath $Executable).Path
if (Get-Process -Name deckforge -ErrorAction SilentlyContinue) {
    throw 'DeckForge is already running. Quit it before starting an isolated smoke test.'
}
$overrides = @{
    APPDATA = (Join-Path $DataRoot 'Roaming')
    LOCALAPPDATA = (Join-Path $DataRoot 'Local')
    USERPROFILE = (Join-Path $DataRoot 'User')
    WEBVIEW2_USER_DATA_FOLDER = (Join-Path $DataRoot 'WebView2')
}
foreach ($directory in $overrides.Values) { New-Item -ItemType Directory -Path $directory -Force | Out-Null }
New-Item -ItemType Directory -Path (Join-Path $overrides.USERPROFILE 'Documents') -Force | Out-Null
$savedEnvironment = @{}
try {
    foreach ($name in $overrides.Keys) {
        $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
        [Environment]::SetEnvironmentVariable($name, $overrides[$name], 'Process')
    }
    $testProcess = Start-Process -FilePath $Executable -ArgumentList '--start-paused' -WorkingDirectory $repoRoot -WindowStyle Hidden -PassThru
    [pscustomobject]@{ ProcessId = $testProcess.Id; DataRoot = $DataRoot; Executable = $Executable }
} finally {
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
}
