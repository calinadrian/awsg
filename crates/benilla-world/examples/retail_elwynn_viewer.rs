//! Inspect the imported arena without a game, player or combat simulation.
//! `WOW_DATA=... WOW_WORLDVIEW_AT=-9396,-633,68 cargo run -p benilla-world --example retail_elwynn_viewer`
fn main() -> bevy::app::AppExit {
    benilla_world::worldview::run(benilla_world::build_id::BuildId {
        version: env!("CARGO_PKG_VERSION"),
        profile: "retail-elwynn-viewer",
        extended: true,
        ..Default::default()
    })
}
