param([Parameter(Mandatory = $true)][string]$Binary)
$ErrorActionPreference = 'Stop'
$originalEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$originalHome = $env:USERPROFILE
$originalLocation = Get-Location
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('hop-test-' + [guid]::NewGuid())
try {
    # Build Unicode at runtime so this also works with Windows PowerShell 5.1's
    # default encoding for BOM-less script files.
    $unicodeName = -join ([char[]](0x043F, 0x0440, 0x043E, 0x0435, 0x043A, 0x0442))
    $project = Join-Path $testRoot "work\alex's $unicodeName [1]"
    New-Item -ItemType Directory -Path (Join-Path $project '.git') -Force | Out-Null
    $junction = Join-Path $testRoot 'work\junction'
    # A cycle also verifies that discovery does not recurse through junctions.
    New-Item -ItemType Junction -Path $junction -Target $testRoot | Out-Null
    $installed = Join-Path $testRoot "hop's $unicodeName.exe"
    Copy-Item -LiteralPath (Resolve-Path $Binary).Path -Destination $installed
    $env:USERPROFILE = $testRoot
    & $installed config --root (Join-Path $testRoot 'work') --no-color
    if ($LASTEXITCODE -ne 0) { throw 'Config failed' }
    $configText = Get-Content -LiteralPath (Join-Path $testRoot '.x-cli-hop\config.toml') -Raw
    if ($configText -match 'junction') { throw 'Discovery followed a junction' }
    & $installed --shell-init | Out-String | Invoke-Expression
    hop A1 --no-color
    if ((Get-Location).Path -ne $project) { throw 'Shell did not change directory' }
    hop A1 --no-color
    $history = Get-Content -LiteralPath (Join-Path $testRoot '.x-cli-hop\history.toml') -Raw
    if ($history -notmatch 'jumps = 2') { throw 'Repeated jumps did not update history' }
    $output = hop list --frequent --no-color
    if ($null -ne $output) { throw 'List polluted stdout' }
    if ((Get-Location).Path -ne $project) { throw 'List changed directory' }
    $ErrorActionPreference = 'Continue'
    hop Z999 --no-color
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -eq 0) { throw 'Invalid selector succeeded' }
    if ((Get-Location).Path -ne $project) { throw 'Failed jump changed directory' }
    # PowerShell expands ~ to its own real home before native invocation.
    # Pass the expanded isolated home so this test never touches runner state.
    hop $testRoot --no-color
    if ((Get-Location).Path -ne (Join-Path $testRoot '.x-cli-hop')) { throw 'Home shortcut failed' }
    if ((Get-Content -LiteralPath (Join-Path $testRoot '.x-cli-hop\history.toml') -Raw) -ne $history) {
        throw 'Non-project actions changed history'
    }
    $global:LASTEXITCODE = 0
} finally {
    Set-Location -LiteralPath $originalLocation.Path
    [Console]::OutputEncoding = $originalEncoding
    $env:USERPROFILE = $originalHome
    if ($junction -and (Test-Path -LiteralPath $junction)) { [IO.Directory]::Delete($junction) }
    Remove-Item Function:\hop -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}
