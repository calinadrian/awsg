//! A little separation between bosses' major warnings; their cooldowns remain independent.
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct Windups {
    last: f32,
    next: f32,
}

impl Windups {
    pub(super) fn available(&self, now: f32) -> bool {
        // A new run rewinds the encounter clock.
        now < self.last || now >= self.next
    }

    pub(super) fn take(&mut self, now: f32) {
        self.last = now;
        self.next = now + 0.7;
    }
}
