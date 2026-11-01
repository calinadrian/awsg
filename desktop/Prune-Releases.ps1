[CmdletBinding()]
param(
    [string]$Repository = 'calinadrian/awsg',
    [ValidateRange(1,100)][int]$Keep = 3,
    [Parameter(Mandatory=$true)][string]$ExpectedLatest
)
$ErrorActionPreference = 'Stop'
$json = & gh api --paginate --slurp "repos/$Repository/releases"
if ($LASTEXITCODE -ne 0) { throw 'Could not list releases; nothing was removed.' }
$releases = @(
    foreach ($page in ($json | ConvertFrom-Json)) {
        foreach ($release in $page) {
            if (!$release.draft -and !$release.prerelease -and $release.tag_name -match '^desktop-v(\d+\.\d+\.\d+\.\d+)$') {
                [PSCustomObject]@{ Tag = $release.tag_name; Version = [Version]$Matches[1] }
            }
        }
    }
) | Sort-Object Version -Descending
if (!$releases -or $releases[0].Tag -ne $ExpectedLatest) { throw 'The expected newest desktop release is not published; nothing was removed.' }
Write-Output ('Keeping: ' + (($releases | Select-Object -First $Keep).Tag -join ', '))
foreach ($release in ($releases | Select-Object -Skip $Keep)) {
    # Deleting a release removes its uploaded assets; retain the lightweight Git tag.
    & gh release delete $release.Tag --repo $Repository --yes
    if ($LASTEXITCODE -ne 0) { throw "Could not remove release $($release.Tag)." }
    Write-Output "Removed release and assets: $($release.Tag)"
}
