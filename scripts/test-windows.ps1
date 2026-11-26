param([Parameter(Mandatory = $true)][string]$Binary)
$ErrorActionPreference = 'Stop'
$originalEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$originalHome = $env:USERPROFILE
$originalPath = $env:Path
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
    $configText = Get-Content -LiteralPath (Join-Path $testRoot '.hop\config.toml') -Raw
    if ($configText -match 'junction') { throw 'Discovery followed a junction' }
    & $installed --shell-init | Out-String | Invoke-Expression
    hop A1 --no-color
    if ((Get-Location).Path -ne $project) { throw 'Shell did not change directory' }
    hop A1 --no-color
    $history = Get-Content -LiteralPath (Join-Path $testRoot '.hop\history.toml') -Raw
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
    if ((Get-Location).Path -ne (Join-Path $testRoot '.hop')) { throw 'Home shortcut failed' }
    if ((Get-Content -LiteralPath (Join-Path $testRoot '.hop\history.toml') -Raw) -ne $history) {
        throw 'Non-project actions changed history'
    }

    $routingDir = Join-Path $testRoot '.hop\routing'
    New-Item -ItemType Directory -Path $routingDir | Out-Null
    $workspace = Join-Path $testRoot 'work'
    $sandbox = Join-Path $workspace 'Sandbox'
    $reserved = Join-Path $workspace 'website'
    New-Item -ItemType Directory -Path (Join-Path $sandbox '.git'), $reserved | Out-Null
    $workspaceToml = $workspace.Replace('\', '/')
    $reservedToml = $reserved.Replace('\', '/')
    $item = Split-Path -Leaf $project
    $personal = @"
version = 5

[[sections]]
root = "$workspaceToml"
items = ["$item", "Sandbox"]

[[sections]]
root = "$reservedToml"
jump_root = true
items = []
"@
    $utf8 = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText((Join-Path $routingDir 'personal.toml'), $personal, $utf8)
    [IO.File]::WriteAllText((Join-Path $routingDir 'work.toml'), 'version = 5', $utf8)
    hop Sandbox --no-color
    if ((Get-Location).Path -ne $sandbox) { throw 'Named jump through Personal routing failed' }
    hop website --no-color
    if ((Get-Location).Path -ne $reserved) { throw 'Named jump to a reserved section root failed' }
    hop --frequent 1 --no-color
    if ((Get-Location).Path -ne $project) { throw 'Frequent navigation with Personal routing failed' }
    $customHome = Join-Path $testRoot "alex's Tool Belt"
    $customBin = Join-Path $customHome 'bin'
    $customHop = Join-Path $customBin 'hop.exe'
    New-Item -ItemType Directory -Path $customBin | Out-Null
    New-Item -ItemType File -Path (Join-Path $customHome '.hop-install') | Out-Null
    Copy-Item -LiteralPath $installed -Destination $customHop
    & $customHop --shell-init | Out-String | Invoke-Expression
    & $customHop --shell-init | Out-String | Invoke-Expression
    if ((@($env:Path -split ';' | Where-Object { $_ -eq $customBin })).Count -ne 1) {
        throw 'Custom install did not appear exactly once in PATH'
    }
    hop $testRoot --no-color
    if ((Get-Location).Path -ne $customHome) { throw 'Custom home shortcut failed' }
    $global:LASTEXITCODE = 0
} finally {
    Set-Location -LiteralPath $originalLocation.Path
    [Console]::OutputEncoding = $originalEncoding
    $env:USERPROFILE = $originalHome
    $env:Path = $originalPath
    if ($junction -and (Test-Path -LiteralPath $junction)) { [IO.Directory]::Delete($junction) }
    Remove-Item Function:\hop -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}
