// Generated from survivors/spell_data.rs, data.rs, and reward_axes.rs.
window.BALANCE_DATA = {
  "classes": [
    {
      "name": "Warrior",
      "start": "cleave",
      "damageScale": 0.81,
      "spells": [
        "cleave",
        "whirlwind",
        "thunder_clap",
        "mortal_strike",
        "execute",
        "demoralizing_shout",
        "battle_shout",
        "bloodthirst",
        "rend",
        "shield_slam",
        "bloodrage"
      ]
    },
    {
      "name": "Paladin",
      "start": "consecration",
      "damageScale": 1.05,
      "spells": [
        "consecration",
        "exorcism",
        "hammer_of_wrath",
        "holy_shock",
        "holy_wrath",
        "judgement",
        "hammer_of_justice",
        "holy_light",
        "divine_shield",
        "holy_shield",
        "redemption"
      ]
    },
    {
      "name": "Hunter",
      "start": "auto_shot",
      "damageScale": 0.86,
      "spells": [
        "auto_shot",
        "arcane_shot",
        "multi_shot",
        "serpent_sting",
        "volley",
        "explosive_trap",
        "concussive_shot",
        "aimed_shot",
        "immolation_trap",
        "frost_trap",
        "hunters_mark"
      ]
    },
    {
      "name": "Rogue",
      "start": "sinister_strike",
      "damageScale": 1.11,
      "spells": [
        "sinister_strike",
        "eviscerate",
        "blade_flurry",
        "rupture",
        "kidney_shot",
        "gouge",
        "ghostly_strike",
        "adrenaline_rush",
        "backstab",
        "slice_and_dice",
        "seal_fate"
      ]
    },
    {
      "name": "Priest",
      "start": "smite",
      "damageScale": 1.11,
      "spells": [
        "smite",
        "shadow_word_pain",
        "mind_blast",
        "holy_nova",
        "power_word_shield",
        "renew",
        "holy_fire",
        "mind_flay",
        "psychic_scream",
        "devouring_plague",
        "lightwell"
      ]
    },
    {
      "name": "Shaman",
      "start": "lightning_bolt",
      "damageScale": 0.95,
      "spells": [
        "lightning_bolt",
        "chain_lightning",
        "earth_shock",
        "flame_shock",
        "frost_shock",
        "lightning_shield",
        "magma_totem",
        "searing_totem",
        "fire_nova",
        "healing_wave",
        "call_of_flame"
      ]
    },
    {
      "name": "Mage",
      "start": "fireball",
      "damageScale": 1.01,
      "spells": [
        "fireball",
        "frostbolt",
        "arcane_missiles",
        "frost_nova",
        "arcane_explosion",
        "blizzard",
        "flamestrike",
        "fire_blast",
        "cone_of_cold",
        "blast_wave",
        "pyroblast",
        "ice_barrier",
        "polymorph"
      ]
    },
    {
      "name": "Warlock",
      "start": "shadow_bolt",
      "damageScale": 1.37,
      "spells": [
        "shadow_bolt",
        "corruption",
        "immolate",
        "curse_of_agony",
        "rain_of_fire",
        "hellfire",
        "death_coil",
        "searing_pain",
        "drain_life",
        "fear",
        "soul_fire",
        "demonic_empowerment"
      ]
    },
    {
      "name": "Druid",
      "start": "wrath",
      "damageScale": 0.93,
      "spells": [
        "wrath",
        "moonfire",
        "starfire",
        "hurricane",
        "insect_swarm",
        "entangling_roots",
        "rejuvenation",
        "regrowth",
        "thorns",
        "swipe",
        "faerie_fire",
        "shred"
      ]
    }
  ],
  "spells": {
    "cleave": {
      "id": 845,
      "kind": "Melee",
      "damage": 47,
      "dot": 0,
      "cooldown": 1.5,
      "radius": 0,
      "range": 7,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "whirlwind": {
      "id": 1680,
      "kind": "Nova",
      "damage": 48,
      "dot": 0,
      "cooldown": 4.5,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "thunder_clap": {
      "id": 6343,
      "kind": "Nova",
      "damage": 21,
      "dot": 0,
      "cooldown": 5,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "mortal_strike": {
      "id": 12294,
      "kind": "Melee",
      "damage": 136.5,
      "dot": 0,
      "cooldown": 3,
      "radius": 0,
      "range": 7,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "execute": {
      "id": 5308,
      "kind": "Execute",
      "damage": 117,
      "dot": 0,
      "cooldown": 2.5,
      "radius": 0,
      "range": 10,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 7,
      "school": 0,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "demoralizing_shout": {
      "id": 1160,
      "kind": "Nova",
      "damage": 8,
      "dot": 0,
      "cooldown": 10,
      "radius": 14,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "cooldown",
        "area"
      ]
    },
    "battle_shout": {
      "id": 6673,
      "kind": "Buff",
      "damage": 0.4,
      "dot": 0,
      "cooldown": 16,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "bloodthirst": {
      "id": 23881,
      "kind": "Melee",
      "damage": 88,
      "dot": 0,
      "cooldown": 4,
      "radius": 0,
      "range": 7,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "rend": {
      "id": 772,
      "kind": "Dot",
      "damage": 0,
      "dot": 15.6,
      "cooldown": 3,
      "radius": 0,
      "range": 8,
      "count": 3,
      "duration": 5,
      "ticks": 5,
      "native": 6,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "shield_slam": {
      "id": 23922,
      "kind": "Melee",
      "damage": 150,
      "dot": 0,
      "cooldown": 6,
      "radius": 0,
      "range": 7,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "bloodrage": {
      "id": 2687,
      "kind": "Passive",
      "damage": 145,
      "dot": 0,
      "cooldown": 7.15,
      "radius": 9,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "consecration": {
      "id": 26573,
      "kind": "GroundSelf",
      "damage": 96,
      "dot": 0,
      "cooldown": 8,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 8,
      "native": 3,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "exorcism": {
      "id": 879,
      "kind": "Bolt",
      "damage": 135,
      "dot": 0,
      "cooldown": 3.5,
      "radius": 0,
      "range": 25,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "hammer_of_wrath": {
      "id": 24275,
      "kind": "Bolt",
      "damage": 66.5,
      "dot": 0,
      "cooldown": 1.6,
      "radius": 0,
      "range": 28,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "holy_shock": {
      "id": 25912,
      "kind": "Missiles",
      "damage": 83.5,
      "dot": 0,
      "cooldown": 3,
      "radius": 0,
      "range": 22,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 1,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "holy_wrath": {
      "id": 2812,
      "kind": "Nova",
      "damage": 56.099998,
      "dot": 0,
      "cooldown": 7,
      "radius": 12,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "judgement": {
      "id": 20271,
      "kind": "Bolt",
      "damage": 200,
      "dot": 0,
      "cooldown": 6,
      "radius": 0,
      "range": 12,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "hammer_of_justice": {
      "id": 853,
      "kind": "Bolt",
      "damage": 55,
      "dot": 0,
      "cooldown": 7,
      "radius": 0,
      "range": 12,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "holy_light": {
      "id": 635,
      "kind": "Heal",
      "damage": 0.3,
      "dot": 0,
      "cooldown": 12,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "divine_shield": {
      "id": 642,
      "kind": "Immune",
      "damage": 0,
      "dot": 0,
      "cooldown": 32,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 4,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "holy_shield": {
      "id": 20925,
      "kind": "Thorns",
      "damage": 30.5,
      "dot": 0,
      "cooldown": 14,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 10,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "duration"
      ]
    },
    "redemption": {
      "id": 7328,
      "kind": "Passive",
      "damage": 52,
      "dot": 0,
      "cooldown": 0.36,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "auto_shot": {
      "id": 75,
      "kind": "Bolt",
      "damage": 34.5,
      "dot": 0,
      "cooldown": 1.1,
      "radius": 0,
      "range": 32,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "arcane_shot": {
      "id": 3044,
      "kind": "Bolt",
      "damage": 68.5,
      "dot": 0,
      "cooldown": 2.5,
      "radius": 0,
      "range": 32,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "multi_shot": {
      "id": 2643,
      "kind": "Bolt",
      "damage": 30,
      "dot": 0,
      "cooldown": 1.6,
      "radius": 0,
      "range": 30,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "serpent_sting": {
      "id": 1978,
      "kind": "Dot",
      "damage": 0,
      "dot": 11.4,
      "cooldown": 2.5,
      "radius": 0,
      "range": 30,
      "count": 2,
      "duration": 5,
      "ticks": 5,
      "native": 6,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "volley": {
      "id": 1510,
      "kind": "Ground",
      "damage": 51.599998,
      "dot": 0,
      "cooldown": 8,
      "radius": 8,
      "range": 28,
      "count": 0,
      "duration": 6,
      "ticks": 6,
      "native": 3,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "explosive_trap": {
      "id": 13812,
      "kind": "Nova",
      "damage": 34.799999,
      "dot": 0,
      "cooldown": 6,
      "radius": 9,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "concussive_shot": {
      "id": 5116,
      "kind": "Bolt",
      "damage": 17,
      "dot": 0,
      "cooldown": 4,
      "radius": 0,
      "range": 30,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "aimed_shot": {
      "id": 19434,
      "kind": "Bolt",
      "damage": 340,
      "dot": 0,
      "cooldown": 4.5,
      "radius": 0,
      "range": 35,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "immolation_trap": {
      "id": 13797,
      "kind": "GroundSelf",
      "damage": 60,
      "dot": 0,
      "cooldown": 9,
      "radius": 7,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 4,
      "native": 3,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "frost_trap": {
      "id": 13810,
      "kind": "GroundSelf",
      "damage": 16.5,
      "dot": 0,
      "cooldown": 10,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 6,
      "ticks": 3,
      "native": 3,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "hunters_mark": {
      "id": 1130,
      "kind": "Passive",
      "damage": 250,
      "dot": 0,
      "cooldown": 1.43,
      "radius": 8,
      "range": 35,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "sinister_strike": {
      "id": 1752,
      "kind": "Melee",
      "damage": 35,
      "dot": 0,
      "cooldown": 1,
      "radius": 0,
      "range": 7,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "eviscerate": {
      "id": 2098,
      "kind": "Execute",
      "damage": 91,
      "dot": 0,
      "cooldown": 3,
      "radius": 0,
      "range": 8,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 7,
      "school": 0,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "blade_flurry": {
      "id": 13877,
      "kind": "Nova",
      "damage": 24,
      "dot": 0,
      "cooldown": 2.5,
      "radius": 7,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "rupture": {
      "id": 1943,
      "kind": "Dot",
      "damage": 0,
      "dot": 15.25,
      "cooldown": 3.5,
      "radius": 0,
      "range": 8,
      "count": 2,
      "duration": 4,
      "ticks": 4,
      "native": 6,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "kidney_shot": {
      "id": 408,
      "kind": "Melee",
      "damage": 25,
      "dot": 0,
      "cooldown": 6,
      "radius": 0,
      "range": 7,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "gouge": {
      "id": 1776,
      "kind": "Melee",
      "damage": 25.5,
      "dot": 0,
      "cooldown": 5,
      "radius": 0,
      "range": 7,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "ghostly_strike": {
      "id": 14278,
      "kind": "Melee",
      "damage": 46,
      "dot": 0,
      "cooldown": 4,
      "radius": 0,
      "range": 7,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "adrenaline_rush": {
      "id": 13750,
      "kind": "Buff",
      "damage": 0.6,
      "dot": 0,
      "cooldown": 20,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "backstab": {
      "id": 53,
      "kind": "Execute",
      "damage": 55.5,
      "dot": 0,
      "cooldown": 1.8,
      "radius": 0,
      "range": 7,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 7,
      "school": 0,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "slice_and_dice": {
      "id": 5171,
      "kind": "Buff",
      "damage": 0.32,
      "dot": 0,
      "cooldown": 15,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 9,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "seal_fate": {
      "id": 14186,
      "kind": "Passive",
      "damage": 130,
      "dot": 0,
      "cooldown": 1.79,
      "radius": 0,
      "range": 8,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "smite": {
      "id": 585,
      "kind": "Bolt",
      "damage": 41.5,
      "dot": 0,
      "cooldown": 1.2,
      "radius": 0,
      "range": 28,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "shadow_word_pain": {
      "id": 589,
      "kind": "Dot",
      "damage": 0,
      "dot": 12,
      "cooldown": 1.6,
      "radius": 0,
      "range": 28,
      "count": 2,
      "duration": 6,
      "ticks": 6,
      "native": 6,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "mind_blast": {
      "id": 8092,
      "kind": "Bolt",
      "damage": 93,
      "dot": 0,
      "cooldown": 4,
      "radius": 0,
      "range": 28,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "holy_nova": {
      "id": 15237,
      "kind": "Nova",
      "damage": 27.299999,
      "dot": 0,
      "cooldown": 4,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "power_word_shield": {
      "id": 17,
      "kind": "Shield",
      "damage": 0.32,
      "dot": 0,
      "cooldown": 12,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 12,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "renew": {
      "id": 139,
      "kind": "Heal",
      "damage": 0.36,
      "dot": 0,
      "cooldown": 10,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 10,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "holy_fire": {
      "id": 14914,
      "kind": "Bolt",
      "damage": 160,
      "dot": 0,
      "cooldown": 5,
      "radius": 0,
      "range": 28,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "mind_flay": {
      "id": 15407,
      "kind": "Cone",
      "damage": 52.799999,
      "dot": 0,
      "cooldown": 2.5,
      "radius": 0,
      "range": 16,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 8,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "psychic_scream": {
      "id": 8122,
      "kind": "Nova",
      "damage": 10.5,
      "dot": 0,
      "cooldown": 12,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 5,
      "axes": [
        "cooldown",
        "area"
      ]
    },
    "devouring_plague": {
      "id": 2944,
      "kind": "Dot",
      "damage": 0,
      "dot": 17.75,
      "cooldown": 6,
      "radius": 0,
      "range": 28,
      "count": 3,
      "duration": 6,
      "ticks": 6,
      "native": 6,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "lightwell": {
      "id": 724,
      "kind": "Totem",
      "damage": 80,
      "dot": 0,
      "cooldown": 14.3,
      "radius": 0,
      "range": 25,
      "count": 1,
      "duration": 7.15,
      "ticks": 0,
      "native": 11,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "lightning_bolt": {
      "id": 403,
      "kind": "Bolt",
      "damage": 36.5,
      "dot": 0,
      "cooldown": 1.4,
      "radius": 0,
      "range": 30,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "chain_lightning": {
      "id": 421,
      "kind": "Chain",
      "damage": 39.5,
      "dot": 0,
      "cooldown": 2.4,
      "radius": 12,
      "range": 28,
      "count": 5,
      "duration": 0,
      "ticks": 0,
      "native": 4,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "earth_shock": {
      "id": 8042,
      "kind": "Bolt",
      "damage": 93.5,
      "dot": 0,
      "cooldown": 3,
      "radius": 0,
      "range": 20,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "flame_shock": {
      "id": 8050,
      "kind": "Dot",
      "damage": 0,
      "dot": 13.75,
      "cooldown": 3,
      "radius": 0,
      "range": 20,
      "count": 2,
      "duration": 6,
      "ticks": 6,
      "native": 6,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "frost_shock": {
      "id": 8056,
      "kind": "Bolt",
      "damage": 58,
      "dot": 0,
      "cooldown": 3.5,
      "radius": 0,
      "range": 20,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "lightning_shield": {
      "id": 324,
      "kind": "Thorns",
      "damage": 20.5,
      "dot": 0,
      "cooldown": 15,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 15,
      "ticks": 0,
      "native": 9,
      "school": 3,
      "axes": [
        "power"
      ]
    },
    "magma_totem": {
      "id": 8190,
      "kind": "Totem",
      "damage": 24,
      "dot": 0,
      "cooldown": 12,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 12,
      "ticks": 6,
      "native": 10,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "searing_totem": {
      "id": 3599,
      "kind": "Totem",
      "damage": 34,
      "dot": 0,
      "cooldown": 10,
      "radius": 0,
      "range": 22,
      "count": 1,
      "duration": 15,
      "ticks": 10,
      "native": 10,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "fire_nova": {
      "id": 1535,
      "kind": "Nova",
      "damage": 42,
      "dot": 0,
      "cooldown": 5,
      "radius": 12,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "healing_wave": {
      "id": 331,
      "kind": "Heal",
      "damage": 0.28,
      "dot": 0,
      "cooldown": 12,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 9,
      "school": 3,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "call_of_flame": {
      "id": 16038,
      "kind": "Passive",
      "damage": 450,
      "dot": 0,
      "cooldown": 1.07,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 2,
      "axes": [
        "power",
        "area"
      ]
    },
    "fireball": {
      "id": 133,
      "kind": "Bolt",
      "damage": 50.5,
      "dot": 0,
      "cooldown": 1.3,
      "radius": 0,
      "range": 30,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "frostbolt": {
      "id": 116,
      "kind": "Bolt",
      "damage": 36.5,
      "dot": 0,
      "cooldown": 1.5,
      "radius": 0,
      "range": 30,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "arcane_missiles": {
      "id": 7269,
      "kind": "Missiles",
      "damage": 25.200001,
      "dot": 0,
      "cooldown": 1.8,
      "radius": 0,
      "range": 28,
      "count": 4,
      "duration": 0,
      "ticks": 0,
      "native": 1,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "frost_nova": {
      "id": 122,
      "kind": "Nova",
      "damage": 12.3,
      "dot": 0,
      "cooldown": 7,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "arcane_explosion": {
      "id": 1449,
      "kind": "Nova",
      "damage": 25.200001,
      "dot": 0,
      "cooldown": 2.5,
      "radius": 9,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "blizzard": {
      "id": 10,
      "kind": "Ground",
      "damage": 60.900002,
      "dot": 0,
      "cooldown": 8,
      "radius": 9,
      "range": 30,
      "count": 0,
      "duration": 7,
      "ticks": 7,
      "native": 3,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "flamestrike": {
      "id": 2120,
      "kind": "Ground",
      "damage": 53.099998,
      "dot": 0,
      "cooldown": 7,
      "radius": 8,
      "range": 30,
      "count": 0,
      "duration": 6,
      "ticks": 3,
      "native": 3,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "fire_blast": {
      "id": 2136,
      "kind": "Missiles",
      "damage": 68,
      "dot": 0,
      "cooldown": 3,
      "radius": 0,
      "range": 22,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 1,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "cone_of_cold": {
      "id": 120,
      "kind": "Cone",
      "damage": 34.799999,
      "dot": 0,
      "cooldown": 4,
      "radius": 0,
      "range": 12,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 8,
      "school": 4,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "blast_wave": {
      "id": 11113,
      "kind": "Nova",
      "damage": 33.900002,
      "dot": 0,
      "cooldown": 8,
      "radius": 10,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "pyroblast": {
      "id": 11366,
      "kind": "Bolt",
      "damage": 379.5,
      "dot": 0,
      "cooldown": 7,
      "radius": 0,
      "range": 35,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "ice_barrier": {
      "id": 11426,
      "kind": "Shield",
      "damage": 0.45,
      "dot": 0,
      "cooldown": 15,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 15,
      "ticks": 0,
      "native": 9,
      "school": 4,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "polymorph": {
      "id": 118,
      "kind": "Passive",
      "damage": 115,
      "dot": 0,
      "cooldown": 2.143,
      "radius": 7,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 6,
      "axes": [
        "power",
        "area"
      ]
    },
    "shadow_bolt": {
      "id": 686,
      "kind": "Bolt",
      "damage": 34,
      "dot": 0,
      "cooldown": 1.4,
      "radius": 0,
      "range": 30,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "corruption": {
      "id": 172,
      "kind": "Dot",
      "damage": 0,
      "dot": 10.416667,
      "cooldown": 1.4,
      "radius": 0,
      "range": 30,
      "count": 2,
      "duration": 6,
      "ticks": 6,
      "native": 6,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "immolate": {
      "id": 348,
      "kind": "Dot",
      "damage": 0,
      "dot": 13.8,
      "cooldown": 2.5,
      "radius": 0,
      "range": 30,
      "count": 1,
      "duration": 5,
      "ticks": 5,
      "native": 6,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "curse_of_agony": {
      "id": 980,
      "kind": "Dot",
      "damage": 0,
      "dot": 11.5,
      "cooldown": 3,
      "radius": 0,
      "range": 30,
      "count": 3,
      "duration": 9,
      "ticks": 9,
      "native": 6,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "rain_of_fire": {
      "id": 5740,
      "kind": "Ground",
      "damage": 52.799999,
      "dot": 0,
      "cooldown": 8,
      "radius": 8,
      "range": 30,
      "count": 0,
      "duration": 8,
      "ticks": 4,
      "native": 3,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "hellfire": {
      "id": 1949,
      "kind": "Nova",
      "damage": 12,
      "dot": 0,
      "cooldown": 1.5,
      "radius": 7,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "death_coil": {
      "id": 6789,
      "kind": "Bolt",
      "damage": 75,
      "dot": 0,
      "cooldown": 6,
      "radius": 0,
      "range": 25,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "searing_pain": {
      "id": 5676,
      "kind": "Missiles",
      "damage": 50,
      "dot": 0,
      "cooldown": 1.6,
      "radius": 0,
      "range": 26,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 1,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "drain_life": {
      "id": 689,
      "kind": "Dot",
      "damage": 0,
      "dot": 19.166667,
      "cooldown": 4,
      "radius": 0,
      "range": 22,
      "count": 1,
      "duration": 3,
      "ticks": 3,
      "native": 6,
      "school": 5,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "fear": {
      "id": 5782,
      "kind": "Bolt",
      "damage": 5.5,
      "dot": 0,
      "cooldown": 6,
      "radius": 0,
      "range": 20,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 5,
      "axes": [
        "cooldown",
        "targets"
      ]
    },
    "soul_fire": {
      "id": 6353,
      "kind": "Passive",
      "damage": 600,
      "dot": 0,
      "cooldown": 4.29,
      "radius": 0,
      "range": 35,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 2,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "demonic_empowerment": {
      "id": 18708,
      "kind": "Buff",
      "damage": 0.4,
      "dot": 0,
      "cooldown": 16,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 0,
      "native": 9,
      "school": 5,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "wrath": {
      "id": 5176,
      "kind": "Bolt",
      "damage": 37.5,
      "dot": 0,
      "cooldown": 1.2,
      "radius": 0,
      "range": 30,
      "count": 2,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "moonfire": {
      "id": 8921,
      "kind": "Dot",
      "damage": 0,
      "dot": 14.3,
      "cooldown": 1.8,
      "radius": 0,
      "range": 30,
      "count": 3,
      "duration": 5,
      "ticks": 5,
      "native": 6,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "starfire": {
      "id": 2912,
      "kind": "Bolt",
      "damage": 136,
      "dot": 0,
      "cooldown": 4.5,
      "radius": 0,
      "range": 32,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "hurricane": {
      "id": 16914,
      "kind": "Ground",
      "damage": 68.699997,
      "dot": 0,
      "cooldown": 9,
      "radius": 10,
      "range": 28,
      "count": 0,
      "duration": 8,
      "ticks": 8,
      "native": 3,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "area",
        "duration"
      ]
    },
    "insect_swarm": {
      "id": 5570,
      "kind": "Dot",
      "damage": 0,
      "dot": 11.5,
      "cooldown": 2.5,
      "radius": 0,
      "range": 28,
      "count": 3,
      "duration": 6,
      "ticks": 6,
      "native": 6,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "duration",
        "targets"
      ]
    },
    "entangling_roots": {
      "id": 339,
      "kind": "Bolt",
      "damage": 28.5,
      "dot": 0,
      "cooldown": 5,
      "radius": 0,
      "range": 25,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 0,
      "school": 3,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "rejuvenation": {
      "id": 774,
      "kind": "Heal",
      "damage": 0.3,
      "dot": 0,
      "cooldown": 8,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 12,
      "ticks": 0,
      "native": 9,
      "school": 3,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "regrowth": {
      "id": 8936,
      "kind": "Heal",
      "damage": 0.42,
      "dot": 0,
      "cooldown": 14,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 9,
      "school": 3,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "thorns": {
      "id": 467,
      "kind": "Thorns",
      "damage": 24,
      "dot": 0,
      "cooldown": 20,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 20,
      "ticks": 0,
      "native": 9,
      "school": 3,
      "axes": [
        "power"
      ]
    },
    "swipe": {
      "id": 779,
      "kind": "Nova",
      "damage": 25.799999,
      "dot": 0,
      "cooldown": 3,
      "radius": 7,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "shred": {
      "id": 5221,
      "kind": "Melee",
      "damage": 105,
      "dot": 0,
      "cooldown": 2,
      "radius": 0,
      "range": 7,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 5,
      "school": 0,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "faerie_fire": {
      "id": 770,
      "kind": "Passive",
      "damage": 200,
      "dot": 0,
      "cooldown": 1,
      "radius": 8,
      "range": 30,
      "count": 1,
      "duration": 0,
      "ticks": 0,
      "native": 11,
      "school": 3,
      "axes": [
        "power",
        "targets",
        "area"
      ]
    },
    "perception": {
      "id": 20600,
      "kind": "Buff",
      "damage": 0.15,
      "dot": 0,
      "cooldown": 30,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 10,
      "ticks": 0,
      "native": 9,
      "school": 1,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "blood_fury": {
      "id": 23234,
      "kind": "Buff",
      "damage": 0.35,
      "dot": 0,
      "cooldown": 40,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 12,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "stoneform": {
      "id": 20594,
      "kind": "Shield",
      "damage": 0.3,
      "dot": 0,
      "cooldown": 45,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 8,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "power",
        "cooldown"
      ]
    },
    "shadowmeld": {
      "id": 20580,
      "kind": "Immune",
      "damage": 0,
      "dot": 0,
      "cooldown": 75,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 3,
      "ticks": 0,
      "native": 9,
      "school": 5,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "will_of_the_forsaken": {
      "id": 7744,
      "kind": "Immune",
      "damage": 0,
      "dot": 0,
      "cooldown": 60,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 4,
      "ticks": 0,
      "native": 9,
      "school": 5,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "war_stomp": {
      "id": 20549,
      "kind": "Nova",
      "damage": 12,
      "dot": 0,
      "cooldown": 15,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 0,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "escape_artist": {
      "id": 20589,
      "kind": "Nova",
      "damage": 6,
      "dot": 0,
      "cooldown": 20,
      "radius": 8,
      "range": 0,
      "count": 0,
      "duration": 0,
      "ticks": 0,
      "native": 2,
      "school": 6,
      "axes": [
        "power",
        "cooldown",
        "area"
      ]
    },
    "berserking": {
      "id": 26635,
      "kind": "Buff",
      "damage": 0.3,
      "dot": 0,
      "cooldown": 35,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 10,
      "ticks": 0,
      "native": 9,
      "school": 0,
      "axes": [
        "cooldown",
        "duration"
      ]
    },
    "penance": {
      "id": 25912,
      "kind": "Missiles",
      "damage": 50.5,
      "dot": 0,
      "cooldown": 2.2,
      "radius": 0,
      "range": 26,
      "count": 3,
      "duration": 0,
      "ticks": 0,
      "native": 1,
      "school": 1,
      "axes": [
        "power",
        "cooldown",
        "targets"
      ]
    },
    "arcane_power": {
      "id": 12042,
      "kind": "Buff",
      "damage": 0.35,
      "dot": 0,
      "cooldown": 30,
      "radius": 0,
      "range": 0,
      "count": 0,
      "duration": 12,
      "ticks": 0,
      "native": 9,
      "school": 6,
      "axes": [
        "cooldown",
        "duration"
      ]
    }
  }
};
