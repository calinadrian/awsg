[CmdletBinding()]
param(
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')][string]$Version = '1.0.0.0',
    [string]$GameDirectory = '',
    [string]$OutputDirectory = '',
    [switch]$EmbedData,
    [switch]$SkipGameBuild
)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (!$GameDirectory) { $GameDirectory = Join-Path $projectRoot 'out/elwynn-playable' }
if (!$OutputDirectory) { $OutputDirectory = Join-Path $projectRoot "out/desktop/$Version" }
$parsedVersion = [Version]$Version
foreach ($part in @($parsedVersion.Major, $parsedVersion.Minor, $parsedVersion.Build, $parsedVersion.Revision)) {
    if ($part -lt 0 -or $part -gt 65535) { throw 'All four version components must be between 0 and 65535.' }
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
$outPrefix = [IO.Path]::GetFullPath((Join-Path $projectRoot 'out')).TrimEnd('\') + '\'
if (!$output.StartsWith($outPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Output must be a child of this checkout''s out/ folder.' }
$game = [IO.Path]::GetFullPath($GameDirectory)
New-Item -ItemType Directory -Path $output -Force | Out-Null
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (!(Test-Path -LiteralPath $compiler)) { throw 'The Windows .NET Framework C# compiler was not found.' }
if (!$SkipGameBuild) {
    Push-Location $projectRoot
    try {
        & cargo build --release -p warcraft-survivors -p asr-coop --no-default-features
        if ($LASTEXITCODE -ne 0) { throw 'The Windows game build failed.' }
    } finally { Pop-Location }
}
$gameExe = Join-Path $projectRoot 'target/release/warcraft-survivors.exe'
if (!(Test-Path -LiteralPath $gameExe)) { throw 'Build warcraft-survivors first.' }

# A small, original shield monogram gives setup, shortcuts and maintenance one Windows icon.
Add-Type -AssemblyName System.Drawing
if (!('ASRIconHandle' -as [type])) { Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class ASRIconHandle { [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr handle); }' }
$bitmap = New-Object Drawing.Bitmap 256,256
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([Drawing.Color]::FromArgb(25,24,21))
$points = [Drawing.Point[]]@([Drawing.Point]::new(32,25),[Drawing.Point]::new(224,25),[Drawing.Point]::new(211,160),[Drawing.Point]::new(128,232),[Drawing.Point]::new(45,160))
$brush = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(86,27,18))
$pen = New-Object Drawing.Pen ([Drawing.Color]::FromArgb(235,190,99)),8
$gold = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(245,214,151))
$font = New-Object Drawing.Font 'Georgia',115,([Drawing.FontStyle]::Bold),([Drawing.GraphicsUnit]::Pixel)
$format = New-Object Drawing.StringFormat
$format.Alignment = [Drawing.StringAlignment]::Center
$format.LineAlignment = [Drawing.StringAlignment]::Center
$graphics.FillPolygon($brush,$points); $graphics.DrawPolygon($pen,$points)
$graphics.DrawString('A',$font,$gold,[Drawing.RectangleF]::new(23,33,210,166),$format)
$iconPath = Join-Path $output 'ASRGame.ico'
$handle = $bitmap.GetHicon(); $icon = [Drawing.Icon]::FromHandle($handle)
$iconStream = [IO.File]::Create($iconPath)
try { $icon.Save($iconStream) } finally { $iconStream.Dispose(); $icon.Dispose(); [ASRIconHandle]::DestroyIcon($handle) | Out-Null; $format.Dispose(); $font.Dispose(); $gold.Dispose(); $pen.Dispose(); $brush.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }

$releaseInfo = Join-Path $output 'ReleaseInfo.cs'
@"
using System.Reflection;
[assembly: AssemblyTitle("ASR Game")]
[assembly: AssemblyProduct("ASR Game")]
[assembly: AssemblyCompany("calinadrian")]
[assembly: AssemblyVersion("$Version")]
[assembly: AssemblyFileVersion("$Version")]
namespace ASRDesktop { static class ReleaseInfo { internal const string Version = "$Version"; internal const string Repository = "calinadrian/awsg"; } }
"@ | Set-Content -LiteralPath $releaseInfo -Encoding UTF8
$launcher = Join-Path $output 'ASRGame.exe'
& $compiler /nologo /target:winexe /platform:x64 /optimize+ "/out:$launcher" "/win32icon:$iconPath" "/win32manifest:$(Join-Path $PSScriptRoot 'ASRGame.manifest')" "/reference:System.dll" "/reference:System.Core.dll" "/reference:System.Drawing.dll" "/reference:System.Windows.Forms.dll" "/reference:System.IO.Compression.dll" "/reference:System.IO.Compression.FileSystem.dll" "/reference:System.Web.Extensions.dll" (Join-Path $PSScriptRoot 'ASRGame.cs') $releaseInfo
if ($LASTEXITCODE -ne 0) { throw 'Installer/updater compilation failed.' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$runtimeFiles = [ordered]@{
    'ASRGame.exe' = $launcher
    'warcraft-survivors.exe' = $gameExe
    'CONTROLS.md' = (Join-Path $projectRoot 'CONTROLS.md')
    'COOP.md' = (Join-Path $projectRoot 'docs/COOP.md')
    'LICENSE-MIT' = (Join-Path $projectRoot 'LICENSE-MIT')
    'LICENSE-APACHE' = (Join-Path $projectRoot 'LICENSE-APACHE')
}
$relay = Join-Path $projectRoot 'target/release/asr-coop.exe'
if (Test-Path -LiteralPath $relay) { $runtimeFiles['asr-coop.exe'] = $relay }
# App-local VC++ runtime avoids a separate administrator-level prerequisite installer.
$crtDirectory = $null
if (Test-Path -LiteralPath (Join-Path $game 'vcruntime140.dll')) { $crtDirectory = $game }
if (!$crtDirectory) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and $visualStudio) {
            $redist = Join-Path $visualStudio 'VC/Redist/MSVC'
            foreach ($toolset in @(Get-ChildItem -LiteralPath $redist -Directory | Sort-Object { $candidateVersion = $null; if ([Version]::TryParse($_.Name, [ref]$candidateVersion)) { $candidateVersion } else { [Version]'0.0' } } -Descending)) {
                $x64 = Join-Path $toolset.FullName 'x64'
                foreach ($candidate in @(Get-ChildItem -LiteralPath $x64 -Directory -Filter 'Microsoft.VC*.CRT' -ErrorAction SilentlyContinue)) {
                    if (Test-Path -LiteralPath (Join-Path $candidate.FullName 'vcruntime140.dll')) { $crtDirectory = $candidate.FullName; break }
                }
                if ($crtDirectory) { break }
            }
        }
    }
}
if (!$crtDirectory) { throw 'The x64 VC++ redistributable files were not found. Install the MSVC C++ build tools, or provide app-local CRT DLLs in the player folder.' }
foreach ($dll in @(Get-ChildItem -LiteralPath $crtDirectory -File -Filter '*.dll')) {
    if ($dll.Name -match '^(vcruntime140|msvcp140|concrt140)') { $runtimeFiles[$dll.Name] = $dll.FullName }
}
function New-ASRPackage([string]$Path, [Collections.IDictionary]$Files) {
    $entries = @()
    foreach ($name in $Files.Keys) {
        $source = [string]$Files[$name]
        if (!(Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing package file: $source" }
        $entries += [ordered]@{ path = $name; size = (Get-Item -LiteralPath $source).Length; sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
    $manifest = [ordered]@{ schema = 1; product = 'asr-game-desktop'; version = $Version; files = @($entries) }
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Create, [IO.FileAccess]::Write)
    $archive = [IO.Compression.ZipArchive]::new($stream,[IO.Compression.ZipArchiveMode]::Create,$false)
    try {
        foreach ($name in $Files.Keys) {
            $compression = if ($name.EndsWith('.MPQ',[StringComparison]::OrdinalIgnoreCase)) { [IO.Compression.CompressionLevel]::NoCompression } else { [IO.Compression.CompressionLevel]::Optimal }
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, [string]$Files[$name], [string]$name, $compression) | Out-Null
        }
        $metadata = $archive.CreateEntry('package.json')
        $writer = [IO.StreamWriter]::new($metadata.Open(),[Text.UTF8Encoding]::new($false))
        try { $writer.Write(($manifest | ConvertTo-Json -Depth 6)) } finally { $writer.Dispose() }
    } finally { $archive.Dispose(); $stream.Dispose() }
}
$runtimeZip = Join-Path $output "ASRGame-$Version-win-x64.zip"
New-ASRPackage $runtimeZip $runtimeFiles
$update = [ordered]@{ schema = 1; product = 'asr-game-desktop'; version = $Version; archive = [IO.Path]::GetFileName($runtimeZip); size = (Get-Item -LiteralPath $runtimeZip).Length; sha256 = (Get-FileHash -LiteralPath $runtimeZip -Algorithm SHA256).Hash.ToLowerInvariant() }
[IO.File]::WriteAllText((Join-Path $output 'update.json'), ($update | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))

$setupZip = $runtimeZip
if ($EmbedData) {
    $dataRoot = Join-Path $game 'Data'
    if (!(Test-Path -LiteralPath $dataRoot -PathType Container)) { throw 'The local player Data folder is missing.' }
    $setupFiles = [ordered]@{}
    foreach ($name in $runtimeFiles.Keys) { $setupFiles[$name] = $runtimeFiles[$name] }
    $dataFiles = @(Get-ChildItem -LiteralPath $dataRoot -Filter '*.MPQ' -Recurse -File)
    if ($dataFiles.Count -eq 0) { throw 'No local MPQ archives found.' }
    foreach ($file in $dataFiles) {
        $relative = $file.FullName.Substring($dataRoot.TrimEnd('\').Length + 1).Replace('\','/')
        $setupFiles["Data/$relative"] = $file.FullName
    }
    $setupZip = Join-Path $output 'setup-payload.zip'
    New-ASRPackage $setupZip $setupFiles
}
$setup = Join-Path $output 'ASRGame-Setup.exe'
if ((Get-Item -LiteralPath $setupZip).Length -gt 1800MB) { throw 'The embedded payload is too large. Build without -EmbedData and select an existing Data folder during installation.' }
$destination = [IO.File]::Open($setup,[IO.FileMode]::Create,[IO.FileAccess]::Write)
try {
    foreach ($source in @($launcher,$setupZip)) { $payloadInput = [IO.File]::OpenRead($source); try { $payloadInput.CopyTo($destination) } finally { $payloadInput.Dispose() } }
    $writer = [IO.BinaryWriter]::new($destination,[Text.Encoding]::ASCII,$true)
    $writer.Write([long](Get-Item -LiteralPath $setupZip).Length)
    $writer.Write([Text.Encoding]::ASCII.GetBytes('ASRSETUP1')); $writer.Flush(); $writer.Dispose()
} finally { $destination.Dispose() }
$hash = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath (Join-Path $output 'ASRGame-Setup.exe.sha256') -Encoding ASCII -Value "$hash  ASRGame-Setup.exe"
Write-Output "Installer: $setup"
Write-Output "Runtime update: $runtimeZip"
Write-Output "Manifest: $(Join-Path $output 'update.json')"
