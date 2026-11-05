//! One deadline for every boss, measured on the pausable encounter clock.
use bevy::prelude::*;

use super::{
    Run, coop,
    enemies::{self, Enemy, Script},
    mechanics::Mechanics,
    units::Fx,
};

pub(super) const SECONDS: f32 = 180.0;
pub(super) const DAMAGE: f32 = 6.0;
pub(super) const SPEED: f32 = 1.5;
pub(super) const NAME: &str = "Berserk";
pub(super) const DESCRIPTION: &str = "Three-minute limit exceeded: damage increased by 500% and movement speed increased by 50% until defeated. Stacks with health-based phases such as Blood Frenzy and Last Stand.";

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        tick.after(super::guardians::keep_lairs)
            .before(enemies::move_enemies)
            .run_if(coop::authority)
            .run_if(coop::world_running)
            .run_if(|time: Res<Time>| time.delta_secs() > 0.0)
            .run_if(|loading: Res<crate::loading_screen::LoadingScreen>| !loading.covering()),
    );
}

fn tick(
    mut run: ResMut<Run>,
    mut bosses: Query<(
        Entity,
        &mut Enemy,
        Option<&mut Script>,
        Option<&mut Mechanics>,
    )>,
    mut fx: Fx,
) {
    for (entity, mut enemy, script, mechanics) in &mut bosses {
        if !enemy.boss
            || enemy.dead
            || enemy.hp <= 0.0
            || enemy.timed_enraged
            || run.elapsed < enemy.enrage_at
        {
            continue;
        }
        enemy.timed_enraged = true;
        enemy.damage *= DAMAGE;
        enemy.enrage_pending();
        if let Some(mut script) = script {
            script.enrage_pending();
        }
        if let Some(mut mechanics) = mechanics {
            mechanics.enrage_pending();
        }
        fx.cast(entity, 8599, &[], None);
        run.yell = Some((
            format!("{} goes berserk! +500% damage, +50% speed!", enemy.def.name),
            5.0,
        ));
    }
}
