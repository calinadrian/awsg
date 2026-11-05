// Generated from docs/mocks/reward_cards.html, the approved food table. All values are sample values.
pub(super) static KINDS: [FoodKind; 5] = [
    FoodKind {
        name: "Sweets",
        one: "SWEET",
        slot: "sweets",
        builds: [
            FoodBuild {
                desc: "Pure sugar. Your spells simply hit harder.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE",
                        stat: 0,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Peppermint Drop",
                        icon: "Interface\\Icons\\inv_misc_food_28",
                    },
                    Food {
                        name: "Cherry Lollipop",
                        icon: "Interface\\Icons\\inv_misc_food_26",
                    },
                    Food {
                        name: "Glazed Ring",
                        icon: "Interface\\Icons\\inv_misc_food_34",
                    },
                    Food {
                        name: "Frosted Swirl",
                        icon: "Interface\\Icons\\inv_misc_food_31",
                    },
                    Food {
                        name: "Starlight Cookie",
                        icon: "Interface\\Icons\\inv_misc_food_62",
                    },
                ],
            },
            FoodBuild {
                desc: "Made to be passed around. Your spells hit less hard but strike more enemies.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE",
                        stat: 0,
                        steps: [0.3, 0.4, 0.5, 0.6, 0.8],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "TARGETS",
                        stat: 255,
                        steps: [1.0, 1.0, 1.0, 2.0, 2.0],
                        percent: false,
                        flat: true,
                    },
                ],
                foods: [
                    Food {
                        name: "Candy Corn",
                        icon: "Interface\\Icons\\inv_misc_food_30",
                    },
                    Food {
                        name: "Ribbon Candy",
                        icon: "Interface\\Icons\\inv_holiday_christmas_wrapping_01",
                    },
                    Food {
                        name: "Sweetheart Candy",
                        icon: "Interface\\Icons\\inv_valentinescandy",
                    },
                    Food {
                        name: "Festival Gift Box",
                        icon: "Interface\\Icons\\inv_misc_gift_01",
                    },
                    Food {
                        name: "Winter Cane",
                        icon: "Interface\\Icons\\inv_misc_food_61",
                    },
                ],
            },
            FoodBuild {
                desc: "Rich and oversized. Your spells hit less hard but cover more ground.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE",
                        stat: 0,
                        steps: [0.5, 0.75, 1.0, 1.25, 1.5],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "AREA",
                        stat: 8,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Flaky Croissant",
                        icon: "Interface\\Icons\\inv_misc_food_32",
                    },
                    Food {
                        name: "Honey Bun",
                        icon: "Interface\\Icons\\inv_misc_food_33",
                    },
                    Food {
                        name: "Berry Pie",
                        icon: "Interface\\Icons\\inv_misc_food_10",
                    },
                    Food {
                        name: "Dark Fudge",
                        icon: "Interface\\Icons\\inv_misc_food_27",
                    },
                    Food {
                        name: "Grinning Pumpkin",
                        icon: "Interface\\Icons\\inv_misc_food_59",
                    },
                ],
            },
        ],
    },
    FoodKind {
        name: "Drinks",
        one: "DRINK",
        slot: "drinks",
        builds: [
            FoodBuild {
                desc: "Strong and steadying. Blows simply hurt less.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE REDUCTION",
                        stat: 5,
                        steps: [0.5, 0.75, 1.0, 1.25, 1.5],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Campfire Cocoa",
                        icon: "Interface\\Icons\\inv_drink_15",
                    },
                    Food {
                        name: "Foaming Ale",
                        icon: "Interface\\Icons\\inv_drink_08",
                    },
                    Food {
                        name: "Barrel Stout",
                        icon: "Interface\\Icons\\inv_drink_05",
                    },
                    Food {
                        name: "Iron Stein",
                        icon: "Interface\\Icons\\inv_drink_03",
                    },
                    Food {
                        name: "Spirit Gourd",
                        icon: "Interface\\Icons\\inv_drink_16",
                    },
                ],
            },
            FoodBuild {
                desc: "Soothing all the way down. Blows hurt more than a stout, but wounds slowly close.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE REDUCTION",
                        stat: 5,
                        steps: [0.2, 0.3, 0.4, 0.5, 0.6],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "HEALTH PER SECOND",
                        stat: 7,
                        steps: [0.1, 0.15, 0.2, 0.25, 0.3],
                        percent: false,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Bog Root Tea",
                        icon: "Interface\\Icons\\inv_drink_17",
                    },
                    Food {
                        name: "Spring Water",
                        icon: "Interface\\Icons\\inv_drink_14",
                    },
                    Food {
                        name: "Healer's Flask",
                        icon: "Interface\\Icons\\inv_drink_18",
                    },
                    Food {
                        name: "Aged Red",
                        icon: "Interface\\Icons\\inv_drink_10",
                    },
                    Food {
                        name: "Sacred Vintage",
                        icon: "Interface\\Icons\\inv_drink_11",
                    },
                ],
            },
            FoodBuild {
                desc: "It burns going down. Blows hurt more than a stout, but melee attackers pay for it.",
                bonuses: &[
                    FoodBonus {
                        label: "DAMAGE REDUCTION",
                        stat: 5,
                        steps: [0.25, 0.4, 0.5, 0.6, 0.75],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "REFLECTED MELEE",
                        stat: 20,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Bitter Cider",
                        icon: "Interface\\Icons\\inv_drink_04",
                    },
                    Food {
                        name: "Thorn Wine",
                        icon: "Interface\\Icons\\inv_drink_02",
                    },
                    Food {
                        name: "Clear Moonshine",
                        icon: "Interface\\Icons\\inv_drink_06",
                    },
                    Food {
                        name: "Black Flask",
                        icon: "Interface\\Icons\\inv_drink_01",
                    },
                    Food {
                        name: "Venom Vintage",
                        icon: "Interface\\Icons\\inv_drink_09",
                    },
                ],
            },
        ],
    },
    FoodKind {
        name: "Meat",
        one: "MEAT",
        slot: "meat",
        builds: [
            FoodBuild {
                desc: "Filling and plain. You simply stay on your feet longer.",
                bonuses: &[
                    FoodBonus {
                        label: "MAXIMUM HEALTH",
                        stat: 2,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Spiced Sausage",
                        icon: "Interface\\Icons\\inv_misc_food_53",
                    },
                    Food {
                        name: "Drumstick",
                        icon: "Interface\\Icons\\inv_misc_food_18",
                    },
                    Food {
                        name: "Fire Skewer",
                        icon: "Interface\\Icons\\inv_misc_food_68",
                    },
                    Food {
                        name: "Prime Ribs",
                        icon: "Interface\\Icons\\inv_misc_food_16",
                    },
                    Food {
                        name: "Kingly Roast",
                        icon: "Interface\\Icons\\inv_misc_food_13",
                    },
                ],
            },
            FoodBuild {
                desc: "Barely cooked. You have less health to spare, but your hits feed you.",
                bonuses: &[
                    FoodBonus {
                        label: "MAXIMUM HEALTH",
                        stat: 2,
                        steps: [0.4, 0.6, 0.8, 1.0, 1.2],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "LIFE STEAL",
                        stat: 18,
                        steps: [0.1, 0.15, 0.2, 0.25, 0.3],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Blood Sausage",
                        icon: "Interface\\Icons\\inv_misc_food_66",
                    },
                    Food {
                        name: "Rare Steak",
                        icon: "Interface\\Icons\\inv_misc_food_47",
                    },
                    Food {
                        name: "Butcher's Cut",
                        icon: "Interface\\Icons\\inv_misc_food_70",
                    },
                    Food {
                        name: "Raw Flank",
                        icon: "Interface\\Icons\\inv_misc_food_71",
                    },
                    Food {
                        name: "Beating Heart",
                        icon: "Interface\\Icons\\inv_misc_organ_03",
                    },
                ],
            },
            FoodBuild {
                desc: "Earned on the hunt. You have less health to spare, but every kill patches you up.",
                bonuses: &[
                    FoodBonus {
                        label: "MAXIMUM HEALTH",
                        stat: 2,
                        steps: [0.5, 0.75, 1.0, 1.25, 1.5],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "HEALTH PER KILL",
                        stat: 15,
                        steps: [0.05, 0.075, 0.1, 0.125, 0.15],
                        percent: false,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Smoked Wing",
                        icon: "Interface\\Icons\\inv_misc_food_51",
                    },
                    Food {
                        name: "Salted Ham",
                        icon: "Interface\\Icons\\inv_misc_food_50",
                    },
                    Food {
                        name: "Roast Fowl",
                        icon: "Interface\\Icons\\inv_misc_food_15",
                    },
                    Food {
                        name: "Boar Chops",
                        icon: "Interface\\Icons\\inv_misc_food_48",
                    },
                    Food {
                        name: "Smoking Haunch",
                        icon: "Interface\\Icons\\inv_misc_food_60",
                    },
                ],
            },
        ],
    },
    FoodKind {
        name: "Fish",
        one: "FISH",
        slot: "fish",
        builds: [
            FoodBuild {
                desc: "Clean and sharp. You simply crit more often.",
                bonuses: &[
                    FoodBonus {
                        label: "CRITICAL CHANCE",
                        stat: 3,
                        steps: [0.5, 0.75, 1.0, 1.25, 1.5],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Pink Shrimp",
                        icon: "Interface\\Icons\\inv_misc_fish_15",
                    },
                    Food {
                        name: "Salmon Roll",
                        icon: "Interface\\Icons\\inv_misc_fish_17",
                    },
                    Food {
                        name: "Reef Darter",
                        icon: "Interface\\Icons\\inv_misc_fish_09",
                    },
                    Food {
                        name: "Frostfin",
                        icon: "Interface\\Icons\\inv_misc_fish_20",
                    },
                    Food {
                        name: "Moonfin",
                        icon: "Interface\\Icons\\inv_misc_fish_21",
                    },
                ],
            },
            FoodBuild {
                desc: "All teeth. You crit less often than a keen catch, but your crits land harder.",
                bonuses: &[
                    FoodBonus {
                        label: "CRITICAL CHANCE",
                        stat: 3,
                        steps: [0.2, 0.3, 0.4, 0.5, 0.6],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "CRITICAL DAMAGE",
                        stat: 4,
                        steps: [2.0, 3.0, 4.0, 5.0, 6.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Bog Eel",
                        icon: "Interface\\Icons\\inv_misc_fish_11",
                    },
                    Food {
                        name: "Yellow Snapper",
                        icon: "Interface\\Icons\\inv_misc_fish_03",
                    },
                    Food {
                        name: "Red Crawler",
                        icon: "Interface\\Icons\\inv_misc_fish_14",
                    },
                    Food {
                        name: "Ember Salmon",
                        icon: "Interface\\Icons\\inv_misc_fish_22",
                    },
                    Food {
                        name: "Gnashing Horror",
                        icon: "Interface\\Icons\\inv_misc_fish_27",
                    },
                ],
            },
            FoodBuild {
                desc: "A fisher's story come true. You crit less often than a keen catch, but rewards roll better.",
                bonuses: &[
                    FoodBonus {
                        label: "CRITICAL CHANCE",
                        stat: 3,
                        steps: [0.25, 0.4, 0.5, 0.6, 0.75],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "LUCK",
                        stat: 14,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Bluegill",
                        icon: "Interface\\Icons\\inv_misc_fish_08",
                    },
                    Food {
                        name: "Silver Trout",
                        icon: "Interface\\Icons\\inv_misc_fish_30",
                    },
                    Food {
                        name: "Harlequin",
                        icon: "Interface\\Icons\\inv_misc_fish_10",
                    },
                    Food {
                        name: "Golden Perch",
                        icon: "Interface\\Icons\\inv_misc_fish_19",
                    },
                    Food {
                        name: "Rainbow Starfin",
                        icon: "Interface\\Icons\\inv_misc_fish_04",
                    },
                ],
            },
        ],
    },
    FoodKind {
        name: "Fruit",
        one: "FRUIT",
        slot: "fruit",
        builds: [
            FoodBuild {
                desc: "Bright and quick. Your spells simply come back sooner.",
                bonuses: &[
                    FoodBonus {
                        label: "COOLDOWN REDUCTION",
                        stat: 1,
                        steps: [0.5, 0.75, 1.0, 1.25, 1.5],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Crisp Apple",
                        icon: "Interface\\Icons\\inv_misc_food_19",
                    },
                    Food {
                        name: "Wild Cherries",
                        icon: "Interface\\Icons\\inv_misc_food_40",
                    },
                    Food {
                        name: "Sun Orange",
                        icon: "Interface\\Icons\\inv_misc_food_41",
                    },
                    Food {
                        name: "Vintner's Grapes",
                        icon: "Interface\\Icons\\inv_misc_food_58",
                    },
                    Food {
                        name: "Ember Pomegranate",
                        icon: "Interface\\Icons\\inv_misc_food_43",
                    },
                ],
            },
            FoodBuild {
                desc: "Twice the fruit. Your spells come back slower than fresh picks, but sometimes cast twice.",
                bonuses: &[
                    FoodBonus {
                        label: "COOLDOWN REDUCTION",
                        stat: 1,
                        steps: [0.2, 0.3, 0.4, 0.5, 0.6],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "DOUBLE CAST",
                        stat: 9,
                        steps: [0.3, 0.45, 0.6, 0.75, 0.9],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Ripe Banana",
                        icon: "Interface\\Icons\\inv_misc_food_24",
                    },
                    Food {
                        name: "Banana Bunch",
                        icon: "Interface\\Icons\\inv_misc_food_23",
                    },
                    Food {
                        name: "Honeydew",
                        icon: "Interface\\Icons\\inv_misc_food_46",
                    },
                    Food {
                        name: "Emerald Grapes",
                        icon: "Interface\\Icons\\inv_misc_food_57",
                    },
                    Food {
                        name: "Blood Melon",
                        icon: "Interface\\Icons\\inv_misc_food_22",
                    },
                ],
            },
            FoodBuild {
                desc: "Good for the mind. Your spells come back slower than fresh picks, but you level faster.",
                bonuses: &[
                    FoodBonus {
                        label: "COOLDOWN REDUCTION",
                        stat: 1,
                        steps: [0.25, 0.4, 0.5, 0.6, 0.75],
                        percent: true,
                        flat: false,
                    },
                    FoodBonus {
                        label: "EXPERIENCE",
                        stat: 12,
                        steps: [1.0, 1.5, 2.0, 2.5, 3.0],
                        percent: true,
                        flat: false,
                    },
                ],
                foods: [
                    Food {
                        name: "Green Apple",
                        icon: "Interface\\Icons\\inv_misc_food_21",
                    },
                    Food {
                        name: "Ripe Peach",
                        icon: "Interface\\Icons\\inv_misc_food_42",
                    },
                    Food {
                        name: "Garden Carrot",
                        icon: "Interface\\Icons\\inv_misc_food_54",
                    },
                    Food {
                        name: "Scholar's Radish",
                        icon: "Interface\\Icons\\inv_misc_food_56",
                    },
                    Food {
                        name: "Sage Root",
                        icon: "Interface\\Icons\\inv_misc_food_55",
                    },
                ],
            },
        ],
    },
];
