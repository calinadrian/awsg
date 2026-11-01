//! Harmless `fxview` cast fixture: `WOW_FX_SPELL=<id>` with `WOW_FX_DISPLAY=<caster>`.
//! Sends only visual messages to a stationary point; no combat, damage or server connection.
use bevy::prelude::*;

use super::FxViewState;
use crate::creature_anim::{CastEvent, CastEventKind, PlaySeq, SpellGoTargets};

pub(super) fn plugin(app: &mut App) {
    if let Some(spell) = std::env::var("WOW_FX_SPELL")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        app.insert_resource(Spell(spell)).add_systems(Update, cast);
    }
}

#[derive(Resource)]
struct Spell(u32);

fn cast(
    mut commands: Commands,
    spell: Res<Spell>,
    state: Option<Res<FxViewState>>,
    time: Res<Time>,
    units: Query<(&GlobalTransform, &crate::net::ObjectStore)>,
    mut bones: Query<&mut crate::entities::BoneAttach>,
    mut seq: ResMut<PlaySeq>,
    mut casts: MessageWriter<CastEvent>,
    mut gos: MessageWriter<SpellGoTargets>,
    mut fired: Local<bool>,
) {
    if *fired {
        return;
    }
    let Some(state) = state else { return };
    let Some(t0) = state.attached_at else { return };
    if time.elapsed_secs() - t0 < 1.0 {
        return;
    }
    let Some(caster) = state.root else { return };
    if std::env::var_os("WOW_FX_IMMEDIATE").is_some() {
        commands
            .entity(caster)
            .insert(crate::survivors::spell_flight::ImmediateMissiles);
        if let Ok(mut bones) = bones.get_mut(caster) {
            crate::survivors::spell_flight::complete_markers(&mut bones);
        }
    }
    let Ok((at, _)) = units.get(caster) else {
        return;
    };
    let distance = std::env::var("WOW_FX_TARGET_DISTANCE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(12.0);
    let target_pos = at.translation() + Vec3::new(0.0, 1.0, -distance);
    let target = commands
        .spawn((
            Transform::from_translation(target_pos),
            GlobalTransform::from_translation(target_pos),
        ))
        .id();
    let seq = seq.next();
    casts.write(CastEvent {
        entity: caster,
        spell_id: spell.0,
        kind: CastEventKind::Go,
        seq,
    });
    gos.write(SpellGoTargets {
        caster,
        spell_id: spell.0,
        hits: vec![target],
        misses: Vec::new(),
        dest: None,
        ammo_display_id: None,
        seq,
    });
    info!(
        "projectile fixture: spell={} caster={:?} target={:?} distance={distance}",
        spell.0,
        at.translation(),
        target_pos
    );
    *fired = true;
}
