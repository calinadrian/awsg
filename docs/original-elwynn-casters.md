# Original Elwynn ranged chasers

Static reference: local ASR.exe SHA-256
`08563B3A976BA4BBE9BFED182CBB5E1630656E7A5619353C4CDB36CBEA31824E`.
Addresses below are RVAs. No original executable or assets are distributed.

The ordinary enemy table at `0x6fdadf8` uses 80-byte rows. Only two of the
nine ordinary Elwynn chasers have a ranged record:

| Creature | Unlock | Spell | Stop range | Cast duration | Attack interval | Base damage | Spawn cost |
|---|---|---|---|---|---|---|---|
| Kobold Geomancer | 2:00 | Fireball 133 | 18 yd | 1.8 s | 3.2 s | 6 | 2 |
| Defias Rogue Wizard | 7:30 | Frostbolt 116 | 18 yd | 1.6 s | 3.2 s | 7 | 2 |

The ranged record's last float is a **cast duration**, not a repeat cooldown.
At `0x5c0bf27`, native AI resets the attack timer from the row's swing interval;
at `0x5c0bf55` it separately sets the cast completion deadline from the ranged
record. `0x5c0bcb4` handles completion before releasing the bolt. Our previous
port launched immediately every 1.8/1.6 seconds, producing respectively about
78%/100% more shots over sustained combat. The corrected path winds up, uses
3.2-second attack intervals, and drops an unfinished cast when its caster dies,
is stunned/fears, returns home, or is relocated. Root alone allows casting.

Native AI stops moving at 18 yd and permits starting casts out to 22 yd
(`0x5c0bb6a`). It casts at close range too. Spell-speed lookup `0x5c6a380`
returns 24 yd/s for Fireball and 28 yd/s for Frostbolt. Damage helper
`0x5bd07e0` uses the mob's scaled damage and active weakening, with no extra
ordinary-caster multiplier. Existing shared armor/talent mitigation remains.

The original weighted selector `0x5c8a2e0` gives every unlocked ordinary mob
`min(1 + max(age_minutes, 0) * 0.5, 4) / spawn_cost`. This matches the port's
age-in-seconds implementation; neither ranged entry gets an extra selection
bonus. There is no evidence here for adding a separate ranged population cap.
Matching selection weights does not prove identical visible populations: combat
survival, terrain, and unfinished scripted-event parity affect that outcome.

This documents static source comparison and a compile/build check. Gameplay
validation is left to the user; full combat/difficulty/co-op parity is not claimed.
