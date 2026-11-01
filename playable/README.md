# Play the Elwynn build

Install [Git LFS](https://git-lfs.com/), then clone the repository:

```powershell
git lfs install
git clone https://github.com/calinadrian/awsg.git
cd awsg
git lfs pull
Expand-Archive playable/elwynn-playable.zip -DestinationPath out
./out/elwynn-playable/Play.cmd
```

The ZIP is about 877 MB and includes the Windows executable and local Elwynn
data pack. Playing it does not require Rust. Builds and gameplay testing are
separate: this build compiled successfully, but gameplay has not been verified.
Existing missing source assets are listed in the included manifest.

To work on the source, install Rust and Visual Studio C++ Build Tools, extract
the ZIP as above, then run:

```powershell
$env:WOW_DATA = "$PWD/out/elwynn-playable/Data"
cargo run --release -p warcraft-survivors --no-default-features
```

To regenerate the pack, see [the packaging instructions](../docs/ELWYNN-PACKAGE.md).
The original full-client data and build caches remain ignored.
