[CmdletBinding()]
param(
    # The pack built by scripts/package_elwynn.py.
    [string]$Pack = '',
    [string]$Repository = 'calinadrian/awsg'
)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (!$Pack) { $Pack = Join-Path $projectRoot 'out/elwynn-playable/Data/patch.MPQ' }
$Pack = [IO.Path]::GetFullPath($Pack)
if (!(Test-Path -LiteralPath $Pack -PathType Leaf)) { throw "Build the data pack first: $Pack" }
$size = (Get-Item -LiteralPath $Pack).Length
if ($size -gt 2GB) { throw 'GitHub release assets must be under 2 GB.' }
$sha256 = (Get-FileHash -LiteralPath $Pack -Algorithm SHA256).Hash.ToLowerInvariant()
$id = $sha256.Substring(0, 12)
# A prerelease outside the desktop-v* sequence: never offered as a program update, never pruned.
$tag = "desktop-data-$id"
$asset = "patch-$id.MPQ"
# Windows PowerShell turns a native command's stderr into a terminating error under 'Stop';
# gh reports through stderr, so its calls are judged by exit code alone.
$ErrorActionPreference = 'Continue'
& gh release view $tag --repo $Repository 2>&1 | Out-Null
$exists = $LASTEXITCODE -eq 0
$ErrorActionPreference = 'Stop'
if (!$exists) {
    # Beside the pack, so the upload name is a hard link on the same volume, not a 1.5 GB copy.
    $staging = Join-Path (Split-Path -Parent $Pack) ('.upload-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $staging | Out-Null
    try {
        $upload = Join-Path $staging $asset
        New-Item -ItemType HardLink -Path $upload -Target $Pack | Out-Null
        $ErrorActionPreference = 'Continue'
        & gh release create $tag $upload --repo $Repository --prerelease --title "ASR Game data $id" --notes "Game data pack for ASR Game launchers. Installed automatically; not a program release." 2>&1 | Write-Output
        $created = $LASTEXITCODE -eq 0
        $ErrorActionPreference = 'Stop'
        if (!$created) { throw 'The data pack upload failed.' }
    } finally { Remove-Item -LiteralPath $staging -Recurse -Force }
} else { Write-Output "$tag is already published." }
$pointer = [ordered]@{ path = 'Data/patch.MPQ'; tag = $tag; asset = $asset; size = $size; sha256 = $sha256 }
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'data.json'), ($pointer | ConvertTo-Json) + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "Data pointer: $(Join-Path $PSScriptRoot 'data.json') -> $tag/$asset"
