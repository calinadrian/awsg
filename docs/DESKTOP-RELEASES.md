# Windows installation and GitHub updates

`desktop/Build.ps1` creates a normal Windows `.exe` installer, a small launcher
and a runtime update ZIP. Windows 10/11 uses its built-in .NET Framework; players
do not install Rust, Python, PowerShell modules or a separate .NET runtime.
The native VC++ runtime DLLs are included app-locally from the build tools'
redistributable folder, without a separate prerequisite installer.

## Install and play

Run `ASRGame-Setup.exe`. The default folder is
`%LOCALAPPDATA%\Programs\ASRGame`. Setup creates a Start-menu folder, an optional
desktop shortcut and an entry in Settings → Apps. No administrator rights are
requested. Open **ASR Game** from the shortcut or `ASRGame.exe` directly.

The launcher checks `calinadrian/awsg` for a newer published desktop release.
When one exists it asks whether to install the update before starting the game.
Without an update, or if GitHub cannot be reached, it opens the installed game.
The Start-menu **Check for updates** shortcut checks manually. A manual update
while a match is running takes effect the next time the game is opened; it does
not terminate the match.

The full local installer built with `-EmbedData` includes the existing Elwynn
runtime data. The automatically published runtime installer asks a new player
to select an existing compatible ASR Game `Data` folder. It copies the archives
without modifying that source. Reinstalling an existing managed installation
can reuse its installed data.

## Progress and uninstalling

Installed copies use `%LOCALAPPDATA%\ASRGame\benilla-config` for characters,
talents, progress and settings. These files stay outside the versioned program
files and survive updates and uninstalling. The original ASR installation and
its saves use their own paths.

To carry progress from this recreation's portable build into a fresh install,
close both copies and copy that portable build's `benilla-config` into
`%LOCALAPPDATA%\ASRGame\benilla-config` before playing the installed copy.
The launcher also recovers missing files from `benilla-config` in the installation
root or an older version folder. Talent recovery compares XP per class, preserving
the entry with higher XP and its spent ranks; the current file is backed up before
any merge. It waits until the game is closed. The original binary game's
character-save format is not imported.

Uninstall through Settings → Apps or Start → ASR Game → Uninstall. The
uninstaller removes its own program files, shortcuts and unchanged installed
data; it retains saves and unrelated files. Close the game before uninstalling.

## Build a local installer

```powershell
# Compile the current game and installer/updater, using the local player data.
.\desktop\Build.ps1 -Version 1.0.0.0 -EmbedData
```

Output is `out/desktop/1.0.0.0/`:

- `ASRGame-Setup.exe`: full local installer when `-EmbedData` was supplied.
- `ASRGame.exe`: launcher/updater, also included in each version's runtime ZIP.
- `ASRGame-1.0.0.0-win-x64.zip`: program-only update package.
- `update.json`: runtime ZIP size, SHA-256 and version.
- `ASRGame-Setup.exe.sha256`: installer checksum.

`-GameDirectory` chooses a different existing player folder containing `Data`.
`-SkipGameBuild` packages already-built release executables. Neither switch
launches or tests the game, installs anything, or changes the selected data.
The current artifacts are unsigned; no signing certificate or private key is
stored in the repository.

## Publish future changes

Commit the intended source and merge it into `main`. Once
`.github/workflows/desktop-release.yml` is on `main`, every new push there builds
the Windows client and relay, then publishes a `desktop-v…` GitHub release with
the installer, runtime ZIP, update manifest and checksum. The workflow can also
be run manually from GitHub Actions. It does not run gameplay or installer tests.
Local source edits and local builds alone do not publish updates.

After a successful publication, the workflow and publish script retain only the
three highest published desktop versions. Older releases and their uploaded
assets are removed; Git tags, draft releases and prereleases are retained.
Cleanup checks that the expected newest version is public before deleting anything.

The four-part desktop version increases with the workflow run number and retry
number, starting above the local `1.0.0.0` installer. Desktop release versions
also advance past explicitly numbered desktop releases already on GitHub, so a
manual publication does not stall the automatic channel. These versions
are separate from the engine's Cargo version. Runtime releases do not bundle or
upload game data, player state, local configuration or private keys.

For an explicit version from committed source already present on `origin/main`:

```powershell
.\desktop\Publish.ps1 -Version 1.0.1.0
```

The script refuses an uncommitted tree or a commit absent from `origin/main`.
It builds a program-only installer and publishes assets to a draft first, then
makes the complete release public. Never replace an existing version's files;
publish a higher version instead. A first desktop release must be published
before installed copies can download an update.

## Update transactions

The updater accepts only this repository's published, non-prerelease
`desktop-vMAJOR.MINOR.BUILD.REVISION` releases. It downloads the exact version's
manifest and archive over HTTPS, verifies archive size/SHA-256, checks every
extracted file against the package manifest and rejects unsafe paths or attempts
to replace personal state or runtime data. The GitHub account publishing releases
is the authority for executable updates.

Program files are extracted into a new folder under `versions/`. `current.json`
is replaced atomically only after validation. The current version and the rollback
version named by `previous.json` remain available. On later launcher starts,
obsolete versions are pruned when no installed game or relay is running. Cleanup
removes only unchanged manifest-owned files, skips directory links, and preserves
local saves and unknown files. A failed download/extraction keeps
the existing version selected. Launcher updates live inside the new version;
the root bootstrap delegates to them rather than overwriting an executable
while it is running. `updates.log` lives beside the saved-state folder.

Setup, update and uninstall operations for an installation share a process lock.
Setup stages data replacements and rolls them back if activation fails. Neither
updates nor uninstalling recursively delete arbitrary installation contents.

The installer/updater still require player testing before being treated as a
verified release.
