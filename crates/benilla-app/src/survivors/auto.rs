//! `WOW_SURVIVORS_AUTO=<class name>`: a hands-off run for testing. It picks the class and its first
//! look, begins once the world is up, takes a random card at every level, walks to the nearest
//! gem away from the crowd and logs the run every ten seconds; `WOW_SURVIVORS_SHOTS=<dir>` also
//! saves a screenshot every few seconds, which needs no window focus. A `:idle`, `:all`, `:late` or
//! `:finale` suffix on the class name changes the run, as the fields below say. `WOW_SURVIVORS_MAP=<index or
//! name>` picks the battleground.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use super::enemies::{Enemy, Gem};
use super::hero::Hero;
use super::hud::{self, Offer};
use super::units::flat;
use super::{data, Phase, Rng, Run, Selection};

#[derive(Resource)]
pub(super) struct Auto {
    class: usize,
    /// An index into [`data::MAPS`].
    map: usize,
    /// `<class>:idle`: stand still, to reach the results screen quickly.
    idle: bool,
    /// `<class>:all`: the class's whole ability pool from the first second.
    all: bool,
    /// `<class>:late`: the whole pool at its top level, two thirds of the way in, for the crowd at
    /// its thickest.
    late: bool,
    /// `<class>:finale`: as `late`, ten seconds before the battleground's last boss, which comes at
    /// half its health so its later phases play out; `WOW_SURVIVORS_BOSS_HP=<fraction>` picks another.
    finale: bool,
    boss_hp: f32,
    /// Whether this run got the setup above. It happens on the run's first frame rather than where
    /// the autopilot presses Play, so a run a stray click starts from the menu gets it too.
    begun: bool,
    frames: u32,
    wait: f32,
    shots: Option<std::path::PathBuf>,
    shot_t: f32,
    shot_n: u32,
    log_t: f32,
    dir: Vec3,
    /// The screens already captured once: the menu, the first level-ups, the results.
    seen: Vec<String>,
}

impl Auto {
    /// One screenshot named for this screen, the first time it shows.
    fn capture_once(&mut self, commands: &mut Commands, name: String) {
        if self.seen.contains(&name) {
            return;
        }
        // A browser has no folder to write to: the moment goes to the console instead, for a
        // DevTools driver to screenshot.
        let Some(dir) = &self.shots else {
            info!("survivors auto: capture {name}");
            self.seen.push(name);
            return;
        };
        let path = dir.join(format!("{name}.png"));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        self.seen.push(name);
    }
}

pub(super) fn from_env() -> Option<Auto> {
    let want = crate::webenv::var("WOW_SURVIVORS_AUTO")?;
    let (name, mode) = want.trim().split_once(':').unwrap_or((want.trim(), ""));
    let idle = mode.eq_ignore_ascii_case("idle");
    let finale = mode.eq_ignore_ascii_case("finale");
    let late = finale || mode.eq_ignore_ascii_case("late");
    let all = late || mode.eq_ignore_ascii_case("all");
    let class = data::CLASSES
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(name))
        .unwrap_or(0);
    let map = crate::webenv::var("WOW_SURVIVORS_MAP").map_or(0, |m| {
        let m = m.trim().to_ascii_lowercase();
        m.parse::<usize>()
            .ok()
            .filter(|&i| i < data::MAPS.len())
            .unwrap_or_else(|| {
                data::MAPS
                    .iter()
                    .position(|d| d.name.to_ascii_lowercase().contains(&m))
                    .unwrap_or(0)
            })
    });
    let shots = std::env::var_os("WOW_SURVIVORS_SHOTS").map(std::path::PathBuf::from);
    if let Some(dir) = &shots {
        let _ = std::fs::create_dir_all(dir);
    }
    Some(Auto {
        class,
        map,
        idle,
        all,
        late,
        finale,
        boss_hp: crate::webenv::var("WOW_SURVIVORS_BOSS_HP")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|f| *f > 0.0 && *f <= 1.0)
            .unwrap_or(0.5),
        begun: false,
        frames: 0,
        wait: 0.0,
        shots,
        shot_t: 3.0,
        shot_n: 0,
        log_t: 0.0,
        dir: Vec3::ZERO,
        seen: Vec::new(),
    })
}

/// The walk the autopilot steers, for [`super::hero::drive_hero`].
pub(super) fn steer(auto: &Auto) -> Vec3 {
    auto.dir
}

/// Toward the nearest gem, pushed off by every enemy close enough to bite, drifting home when
/// there is nothing to pick up.
pub(super) fn plan(
    time: Res<Time<Real>>,
    mut auto: ResMut<Auto>,
    selection: Res<Selection>,
    hero: Query<&Transform, With<Hero>>,
    gems: Query<&Transform, (With<Gem>, Without<Hero>)>,
    foes: Query<(&Transform, &Enemy), Without<Hero>>,
    caches: Query<&Transform, With<super::caches::Cache>>,
) {
    let Some(hero) = hero.single().ok().filter(|_| !auto.idle) else {
        auto.dir = Vec3::ZERO;
        return;
    };
    let at = hero.translation;
    let mut want = gems
        .iter()
        .map(|t| flat(t.translation - at))
        .filter(|d| d.length() < 30.0)
        .min_by(|a, b| a.length().total_cmp(&b.length()))
        .map_or(Vec3::ZERO, |d| d.normalize_or_zero());
    if crate::webenv::var("WOW_SURVIVORS_CACHE_PROBE").is_some() {
        if let Some(cache) = caches.iter().min_by(|a, b| {
            a.translation
                .distance_squared(at)
                .total_cmp(&b.translation.distance_squared(at))
        }) {
            want = flat(cache.translation - at).normalize_or_zero();
        }
    }
    if want == Vec3::ZERO {
        let a = time.elapsed_secs() * 0.35;
        want = Vec3::new(a.cos(), 0.0, a.sin()) * 0.5;
        let map = selection.map_def();
        let home = flat(super::hero::arena(map) - at);
        if home.length() > (map.leash * 0.5).min(40.0) {
            want += home.normalize();
        }
    }
    let mut away = Vec3::ZERO;
    for (t, e) in &foes {
        if e.dead {
            continue;
        }
        let d = flat(at - t.translation);
        let len = d.length();
        let reach = 7.0 + e.radius;
        if len < reach && len > 0.01 {
            away += d / len * (reach - len) / reach * 3.0;
        }
    }
    auto.dir = (want + away).normalize_or_zero();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut auto: ResMut<Auto>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut offer: ResMut<Offer>,
    mut pending: ResMut<super::cards::PendingPick>,
    mut rng: ResMut<Rng>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    heroes: Query<(), With<Hero>>,
    mut enemies: Query<&mut Enemy>,
    gear: Res<super::inventory::Equipment>,
) {
    let picking = crate::webenv::var("WOW_SURVIVORS_REWARD_PREVIEW").as_deref() == Some("chest-pick");
    if picking && *phase.get() == Phase::Playing && run.food.iter().any(Option::is_some) {
        auto.capture_once(&mut commands, "picked".into());
    }
    let dt = time.delta_secs();
    auto.wait += dt;
    auto.frames += 1;
    if *phase.get() == Phase::Menu {
        auto.begun = false;
    } else if !auto.begun && matches!(phase.get(), Phase::Playing | Phase::LevelUp) {
        auto.begun = true;
        begin(&auto, &mut run, selection.map_def().name);
    }
    match phase.get() {
        Phase::Menu => {
            if selection.map != auto.map {
                selection.select_map(auto.map);
                auto.wait = 0.0;
            } else if selection.class != Some(auto.class) {
                selection.class = Some(auto.class);
                selection.looks.clear();
                selection.look = None;
                selection.version += 1;
                auto.wait = 0.0;
            } else if !selection.looks.is_empty() && !selection.can_visit(auto.map) {
                // The class's first look may be the other faction's: take the first race the
                // battleground lets in.
                let zone = data::MAPS[auto.map].name;
                let look = selection.looks.iter().position(|&(_, race, _)| {
                    selection.race_unlocked(race) && super::races::can_enter(race, zone)
                });
                if let Some(look) = look.filter(|&l| selection.look != Some(l)) {
                    selection.look = Some(look);
                    selection.version += 1;
                }
                auto.wait = 0.0;
            } else if heroes.is_empty() || loading.covering() || gear.open {
                auto.wait = 0.0;
            } else if auto.wait > 2.0 && auto.wait < 3.0 {
                auto.capture_once(&mut commands, "menu".into());
            } else if auto.wait > 3.0 && hud::start_run(&mut run, &mut selection) {
                if let Some(kind) = crate::webenv::var("WOW_SURVIVORS_REWARD_PREVIEW") {
                    if kind == "chest-full" {
                        run.food = super::food::all_common();
                    }
                    if kind.starts_with("chest") {
                        run.pending_chests = 1;
                    } else {
                        run.pending_levels = 1;
                    }
                    next.set(Phase::LevelUp);
                } else {
                    next.set(Phase::Playing);
                }
                auto.wait = 0.0;
            }
        }
        Phase::LevelUp => {
            // After the deal-in, so the shot shows the cards settled.
            if auto.wait > 1.2 && (run.level <= 3 || run.level == 10) {
                auto.capture_once(&mut commands, format!("levelup-{}", run.level));
            }
            // The chosen card's moment, then the reward going into its slot.
            if picking && auto.wait > 1.4 {
                pending.begin(0);
            }
            if picking && auto.wait > 1.55 {
                auto.capture_once(&mut commands, "levelup-picking".into());
            }
            if auto.wait > 1.6 && crate::webenv::var("WOW_SURVIVORS_REWARD_PREVIEW").is_none() {
                let n = offer.len().max(1);
                let i = (rng.range(0.0, n as f32) as usize).min(n - 1);
                hud::pick(i, &mut run, &mut offer, &mut rng, &mut next);
                auto.wait = 0.0;
            }
        }
        Phase::GameOver => {
            if auto.wait > 1.5 {
                auto.capture_once(&mut commands, "over".into());
            }
            if auto.wait > 6.0 {
                next.set(Phase::Menu);
                auto.wait = 0.0;
            }
        }
        _ => auto.wait = 0.0,
    }
    if auto.finale && *phase.get() == Phase::Playing {
        for mut e in &mut enemies {
            if e.boss && e.def.last && !e.dead && e.hp >= e.max_hp {
                e.hp = e.max_hp * auto.boss_hp;
            }
        }
    }

    auto.log_t -= dt;
    if auto.log_t <= 0.0 && *phase.get() != Phase::Menu {
        auto.log_t = 10.0;
        let fps = auto.frames as f32 / 10.0;
        auto.frames = 0;
        let alive = enemies.iter().filter(|e| !e.dead).count();
        let build: Vec<String> = run
            .abilities
            .iter()
            .map(|o| format!("{}:{}", o.def.key, o.level))
            .collect();
        info!(
            "survivors auto: {:?} t={:.0}s fps={fps:.0} alive={alive} level={} hp={:.0} kills={} damage={:.0} build=[{}]",
            phase.get(),
            run.elapsed,
            run.level,
            run.hp,
            run.kills,
            run.damage_done,
            build.join(" ")
        );
    }

    if let Some(dir) = auto.shots.clone() {
        auto.shot_t -= dt;
        if auto.shot_t <= 0.0 {
            auto.shot_t = 6.0;
            auto.shot_n += 1;
            let path = dir.join(format!("shot-{:03}.png", auto.shot_n));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
    }
}

/// The run as the suffix asks for, on its first frame.
fn begin(auto: &Auto, run: &mut Run, zone: &str) {
    info!("survivors auto: run begins as {} at {zone}", run.class.name);
    if crate::webenv::var("WOW_SURVIVORS_CACHE_PROBE").is_some() {
        run.gold = 100;
    }
    if auto.all {
        for key in run.class.pool {
            if !run.abilities.iter().any(|o| o.def.key == *key) {
                run.abilities.push(super::Owned::new(
                    data::ability(key),
                    super::rewards::Rarity::Common,
                ));
            }
        }
    }
    if auto.late {
        for o in &mut run.abilities {
            for _ in 1..data::MAX_ABILITY_LEVEL {
                o.upgrade(
                    super::rewards::Rarity::Rare,
                    [Some(super::rewards::Upgrade::Power), None],
                );
            }
        }
        run.elapsed = 600.0;
        run.bosses = 3;
        // About where the XP curve has an ordinary run by then; a level-1 hero folds at once.
        run.level = 20;
    }
    if auto.finale {
        let bosses = &data::roster(auto.map).bosses;
        if let Some(last) = bosses.last() {
            run.elapsed = (last.from - 10.0).max(0.0);
            run.bosses = bosses.len() as u32 - 1;
        }
        run.level = 26;
    }
    if auto.late {
        run.hp = run.stats().max_hp;
    }
}
