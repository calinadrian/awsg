//! The `warcraft-survivors` launcher: the build stamp, handed to the survivors mode.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use benilla_app::BuildId;

fn main() -> benilla_app::AppExit {
    checkout_data();
    benilla_app::run_survivors(BuildId {
        version: env!("CARGO_PKG_VERSION"),
        describe: env!("BENILLA_GIT_DESCRIBE"),
        sha: env!("BENILLA_GIT_SHA"),
        short: env!("BENILLA_GIT_SHORT"),
        date: env!("BENILLA_GIT_DATE"),
        profile: env!("BENILLA_PROFILE"),
        project_dir: env!("BENILLA_PROJECT_DIR"),
        ..Default::default()
    })
}

/// Cargo places this checkout's executable in `target/<profile>`; direct launches need its data too.
fn checkout_data() {
    if std::env::var_os("WOW_DATA").is_some() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    // Packaged installations keep Data beside the executable and use the normal resolver.
    if dir.join("Data").is_dir() || dir.join("WoW/Data").is_dir() {
        return;
    }
    for root in dir.ancestors().take(3) {
        let data = root.join("wow-data/Data");
        if data.join("terrain.MPQ").is_file() {
            std::env::set_var("WOW_DATA", data);
            return;
        }
    }
}
