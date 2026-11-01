[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidatePattern('^\d+\.\d+\.\d+\.\d+$')][string]$Version)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    $dirty = & git status --porcelain
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Commit the intended source changes before publishing. Local installers can be built from an uncommitted tree.' }
    $head = & git rev-parse HEAD
    & git fetch origin main
    if ($LASTEXITCODE -ne 0) { throw 'Could not read origin/main.' }
    & git merge-base --is-ancestor $head origin/main
    if ($LASTEXITCODE -ne 0) { throw 'Publish the source through main first.' }
    & (Join-Path $PSScriptRoot 'Build.ps1') -Version $Version
    $output = Join-Path $projectRoot "out/desktop/$Version"
    $tag = "desktop-v$Version"
    & gh release create $tag --draft --repo calinadrian/awsg --target $head --title "ASR Game $Version" --generate-notes (Join-Path $output 'update.json') (Join-Path $output "ASRGame-$Version-win-x64.zip") (Join-Path $output 'ASRGame-Setup.exe') (Join-Path $output 'ASRGame-Setup.exe.sha256')
    if ($LASTEXITCODE -ne 0) { throw 'GitHub release publication failed.' }
    & gh release edit $tag --repo calinadrian/awsg --draft=false --latest
    if ($LASTEXITCODE -ne 0) { throw 'Assets were uploaded to a draft, but the release could not be published.' }
    & (Join-Path $PSScriptRoot 'Prune-Releases.ps1') -ExpectedLatest $tag
} finally { Pop-Location }
