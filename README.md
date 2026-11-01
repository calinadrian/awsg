# Azeroth Survivors — collaboration source

Our working version of the offline Warcraft survival game, built with Rust and
Bevy on the benilla/wenilla engine. Move, fight waves, collect loot, and choose
upgrades. The native launcher is `warcraft-survivors`; gameplay code lives in
`crates/benilla-app/src/survivors/`.

This repository contains the current source, embedded card UI assets, build
scripts, and vendored dependencies. It starts with a single source snapshot to
keep clones small. Game installations, extracted assets, local patches, build
outputs, previews, logs, saves, and credentials stay outside Git.

## Windows setup

1. Install [Rust](https://rustup.rs/) with the MSVC toolchain and Visual Studio
   Build Tools with **Desktop development with C++**. The pinned Rust version is
   in `rust-toolchain.toml`; rustup selects it when you enter the checkout.
2. Clone this repository:

   ```powershell
   git clone https://github.com/calinadrian/awsg.git
   cd awsg
   ```

3. Supply your own WoW 1.12.1 (build 5875) client data. Put its `Data/*.MPQ`
   files in `wow-data/Data/` in this checkout.
4. Build and run from PowerShell:

   ```powershell
   ./play.cmd
   ```

The first build downloads Rust dependencies and compiles the engine. For a data
folder elsewhere, launch directly instead (`play.cmd` sets its own data path):

```powershell
$env:WOW_DATA = 'D:/Games/WoW/Data'
cargo run --release -p warcraft-survivors
```

This collaboration build currently offers **Elwynn Forest only**, with all
current races and classes. No server or account is needed. Controls are in
[CONTROLS.md](CONTROLS.md).

## Small local player package

To create a separate Elwynn-only player folder from your existing local data:

```powershell
python3.12 scripts/package_elwynn.py --zip
```

Any Python 3.10+ interpreter works; substitute its command if needed. The script
builds the asset reader and player executable, reads your existing MPQ patch
chain, and writes `out/elwynn-playable/` and `out/elwynn-playable.zip`.
Run that folder's `Play.cmd` to play
without Rust. The original `wow-data/Data/` is kept intact.

The pack keeps the arena's nine detailed terrain tiles, nearby coarse horizon
heights, referenced scenery, current character appearances and equipment,
class spell effects, audio, and shared UI. Other detailed zones, dungeons,
cinematics, and the full world-map UI are excluded. `asset-manifest.json` records
included files and any references already missing from the source data. Shared
spell/UI libraries are retained conservatively; this is not a byte-minimal pack.

The player archive contains locally supplied game assets. It remains ignored
under `out/`; keep it out of GitHub. Building and packaging do not verify
gameplay, which is left for manual testing.

## Working together

Pull `main`, create a branch for your change, then open a pull request against
`calinadrian/awsg`:

```powershell
git switch main
git pull --ff-only
git switch -c feature/my-change
# Edit the source, then stage the specific files you changed.
git add path/to/changed-file
git commit -m "Describe the change"
git push -u origin feature/my-change
```

Keep `Cargo.lock`, all workspace crates, and `third_party/`: they are needed for
reproducible dependency resolution and local patched dependencies. Never add
`target/`, `wow-data/`, `out/`, `benilla-config/`, or extracted game assets.

## Local asset differences

The current checkout also uses optional local race and retail Elwynn overlays.
Those archives are excluded, so a fresh clone with base 1.12.1 data may look
different or lack custom race visuals. The import/repair scripts remain here,
but several depend on separate ASR asset tools and a retail installation that
are **not included**. They are not required to compile the Rust source.
See [retail Elwynn](docs/RETAIL-ELWYNN.md) and
[NPC asset staging](docs/RETAIL-ELWYNN-NPCS.md) for their requirements and limits.

The browser path remains available through `scripts/web-setup.sh`,
`scripts/web-build.sh`, and `wenilla-host`; it requires Bash and WebGPU. Start
with the native launcher for this collaboration checkout. Older engine docs and
`AGENTS.md` also describe upstream workflows; this project's collaboration remote
is `calinadrian/awsg`.

## Credits and licenses

Based on [OutThisLife/azeroth-survivors-re](https://github.com/OutThisLife/azeroth-survivors-re),
the local recreation of **Aethus's** fan game
[Azeroth Survivors](https://wow.aethus.pl/), using
[benilla](https://github.com/samwhosung/benilla) and
[wenilla](https://github.com/Arnesen/wenilla).

Engine code retains its [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE)
licenses; vendored components retain their upstream licenses. Card assets are
documented in [their README](crates/benilla-app/assets/cards/README.md).
World of Warcraft and its art belong to Blizzard Entertainment. Screenshots
show user-supplied game content. This fan project is not affiliated with Blizzard
and does not bundle game archives.
