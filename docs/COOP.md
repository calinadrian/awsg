# Player-hosted co-op

The operator runs one directory/relay. Players connect to that server, create
public rooms or private rooms with invite codes, and host matches for up to four
people. A player host owns the encounter; the operator's server routes traffic
between members of that room. Guests need no incoming ports or router setup.

## Run the server

From the source checkout on the operator's PC:

```powershell
cargo build --release -p asr-coop
.\target\release\asr-coop.exe --bind 0.0.0.0:47771
```

The game defaults to `ws://hassian.duckdns.org:47771`. Existing saved localhost
addresses from the initial installer migrate to this address; custom server
addresses and `WOW_ASR_COOP_URL` overrides are preserved.

The relay's default bind, when omitted, is `127.0.0.1:47771`; use the explicit
`--bind 0.0.0.0:47771` above for outside connections. Forward **TCP 47771** on
the router to the operator PC (`192.168.100.10:47771` on the current network)
and allow that listener through Windows Firewall. Keep the PC's LAN address
reserved in the router and DuckDNS pointed at the router's public address.
DNS supplies the address; it does not create the router forwarding rule.

If the router cannot connect to its own public address (NAT loopback), the
operator can set `WOW_ASR_COOP_URL=ws://127.0.0.1:47771` before launching the
game. Friends on other networks keep the default DuckDNS address. Other PCs
on the operator's LAN can use `ws://192.168.100.10:47771` in connection settings.

For TLS, put the relay behind a reverse proxy with a valid certificate, then use
its `wss://` address. For example, an existing Caddy site can route a separate
path to this listener:

```caddyfile
handle /asr-game/coop {
    reverse_proxy 127.0.0.1:47771
}
```

Clients then use `wss://YOUR-HOSTNAME/asr-game/coop`. Keep the certificate and
private key in the proxy's configuration, outside this repository.

This protocol is separate from the original executable's `/coop/lobby/` and
`/coop/relay/` services. Those services can continue running on their existing
ports. ASR-game clients and original ASR clients cannot join each other's rooms.
All participants must run the same ASR-game build.

## Play

1. Select the hero you want to play, then open **Co-op** (or press F6).
2. Enter your name. Choose **Host game** to create a party, or **Join game** and
   enter your friend's six-character code. Connection happens automatically.
3. The host shares the large **Join code**, using **Copy code**. Guests click
   **Ready to play**; the host clicks **Start adventure** once everyone is ready.
4. **Public games** lists open parties. **Invite only** keeps a hosted party off
   that list. The host is ready automatically.

The operator configures the server address under **Connection settings** once.
Friends must use the same server. The server address and player name are
remembered in `benilla-config/survivors-coop.txt`; reconnecting does not require
re-entering either. Ctrl+A selects a field's contents and Ctrl+V pastes a code.

`WOW_ASR_COOP_URL` overrides the remembered server address, which is useful when
distributing a configured build. Elwynn is the available arena. Guests may use
either faction and keep their own class, race, talents and card choices.

The host synchronizes enemies, boss outcomes, the clock and pickups. Collected
XP and gold are shared with living party members, with each hero's own growth
and gold bonuses, divided by the native party share `1 + (living - 1) * 0.336`. Meals heal the player collecting them. Arena caches and
boss/guardian rewards give each player their own choices. In co-op, a card,
item choice or pause menu removes only that player from enemy targeting and
protects them from damage while the encounter and other players continue.
Their automatic attacks wait until they return. Loading screens still hold
the encounter until party members are ready; network traffic uses real time.
Solo reward screens continue to pause the run.
Fallen players cannot heal themselves back to life. A living teammate can stand
within five yards of a corpse for three continuous seconds to resurrect that
player at half health. Leaving range resets progress; reward and pause menus
suspend it. The host ends the run in defeat when everyone falls. A quitting or
crashing host ends the party.

Elwynn's ordinary chasers use the original native game's time-based spawn
budget, kill-pressure feedback, boss suppression and separate population floor.
Living party size scales density and the ordinary constructor's total living
enemy limit: 50 solo, 67 with two, 84 with three and 100 with four. Spawn centers
rotate across living heroes. Timed roaming elites are separate from the fixed
lair guardians.

The main menu also offers **Solo - Low Memory**. This optional solo variant runs
ordinary spawn credits and the population floor at 60%, and reduces the normal
spawn limit from 50 to 30 (18 in compact maps). Ordinary mobs have 1.5x health,
5/3x XP and 5/3x gold per successful gold roll. Fractional rewards use unbiased
rounding; gold drop chance stays the same. At the same spawn-pressure level,
60% density x 5/3 rewards preserves expected XP/gold while 60% x 1.5 HP gives
90% aggregate health. Actual rewards per minute still depend on kill speed and
the spawn cap. Kill-pressure credits are normalized by the same density factor.
Bosses, fixed guardians, wave elites and their rewards retain their normal
stats/cadence. Co-op and Test Grounds always disable this variant. This reduces
active enemies; it does not change loaded textures or world assets.

The native constructors also scale health with the living party, as
`1 + (living - 1) * 0.7 * k`: k = 0.72 for ordinary chasers and roaming elites
(ASR.exe 0.124.5 RVA 0x5c00785, giving 1.50/2.01/2.51x for two to four players)
and k = 1.04 for bosses and lair guardians (0x5c134be, 0x5c6798a: 1.73/2.46/3.18x).
Health is fixed at spawn or a guardian's wake; enemy damage is not party-scaled.
Damage each hero takes is multiplied by `1 + (living - 1) * 0.7 * 0.064`
(0x5be466b), and collected XP and gold use the density share above
(0x5c5a2e1, 0x5bfe22f). A vector of the same factors at 0x5bdc3ab, which may
rescale living enemies when the party changes, is not ported. These source-derived
rules do not establish full terrain, scripted event-wave or difficulty/endless parity.

Party frames show names, levels, health and choosing/loading status beneath
the player frame. Remote heroes also have overhead names and health bars.
The host sends world snapshots at 15 Hz and guests send their pose and hits at
20 Hz, at an even rate rather than resetting each frame. Remote enemies, heroes
and pickups glide from where they are drawn to each new snapshot over one measured
packet interval, extrapolate up to 0.15 s when a packet is late and snap beyond
20 yards; remote heroes animate at the speed being drawn. Guests number their
packets and the host echoes the last one applied, so a guest's hits stay on enemy
health bars until the host confirms them instead of bouncing back for a round
trip. Snapshot rows use short keys, omit defaults and round positions to 1 cm.
Clients use TCP_NODELAY, and snapshot/hit lookups use entity-ID indexes. Actual
latency and frame rate on players' machines still need gameplay confirmation.

## Equipment and shared auctions

The character menu has **Inventory** and **Auction House** buttons. Inventory is
per class and server on this PC; race and gender changes keep that class's gear.
The server owns every item copy, material balance, gold balance and listing.
All players using the same directory see the same marketplace, including when
they play solo. The catalog reuses the owned ASR project's 696 common items and
nine zone tables, with individual bonuses and no set bonuses. Only a playable
zone's own table awards its equipment.

Kills award equipment and materials: normal enemies have an 8% equipment chance,
elites 35%, and bosses 100%. Equipment rolls are filtered by class. Material
chances are 35%/100%/100%, giving 1/3/8 materials. Duplicate copies stay separate.
Gear can be equipped into compatible slots, locked, upgraded with materials
through +5, scrapped, or listed for gold. Listings reserve their item for 48 hours;
buying transfers ownership and gold in one saved transaction. Cancelled and
expired listings return to the seller's bags. Equipped and locked items must be
unequipped and unlocked before sale or scrapping. Two-handed weapons displace
the off hand. Bonuses and outfits are captured when a run starts; co-op also
shares outfits with the other players. Equipping armor uses the character's
live skin composite instead of its baked starter outfit on non-macOS builds.
Appearance fitting still requires player validation.

Kills are submitted in batches during the run, and unspent run gold is banked
when the results open, when leaving to the menu, or on a graceful exit. Spending
run gold on caches reduces that payout. Pending requests are saved before sending
and retried with the same transaction ID after connection loss or a server write
failure. Solo and co-op use one inventory: normal rewards, equipment actions,
achievements, talent experience, builds and completion records commit to the local ledger
even without a server. Reconnecting reconciles changes against the last acknowledged
snapshot, imports server changes, and retains local changes made during the request.
Auctions require a connection and wait for inventory synchronization before trading.
Conflicting edits to the same item or talent build pause synchronization and preserve
both versions rather than overwrite one. The simulation remains player-hosted: bounded reward
reports and duplicate protection do not provide server-authoritative anti-cheat.

Offline synchronization requires matching updated game and directory binaries.
The first local migration preserves the inventory cache and local ledger as
`*.before-offline-sync.bak`; keep these alongside the normal saves. A journalled
acknowledgement and durable transaction IDs allow interrupted imports to resume.
The server also retains an `asr-economy.before-offline-sync.json` snapshot before
accepting its first offline inventory. Existing separate solo and realm inventories
are combined on first connection; subsequent syncs apply changes since acknowledgement.

The updated directory binary includes the economy service on the existing
WebSocket port; it needs no separate Python service or database installation.
The original ASR preview database and its real saves are not imported or changed.
Run **Host Server.cmd** in a source checkout, or rebuild and restart the directory:

```powershell
cargo build --release -p asr-coop
.\target\release\asr-coop.exe --bind 0.0.0.0:47771 --economy-store .\benilla-config\economy\asr-economy
```

The default store is `benilla-config/economy/asr-economy`. Its `.0.json` and
`.1.json` snapshots alternate on each transaction; preserve both and back them
up together while the server is stopped. Use one directory process per store.
The server binds its port before opening the store. Never replace the store with
a fresh copy when updating the executable. Client credentials, cached inventory
and pending transactions are in `benilla-config/survivors-inventory.json`; keep
this file to retain access to those heroes. A server that predates economy
messages must be updated before these screens can synchronize. Public servers
should use the `wss://` proxy configuration above to protect hero credentials.

## Local completion records

Each PC keeps its class XP and talents in `survivors-talents.txt`, using the same
payout path for solo and co-op. `survivors-progress.toml` keeps per-class,
per-zone runs, victories, co-op victories, best survival time and best kills.
Rewards settle once when results open or the player returns to the menu.
Neither the relay nor another player's host can replace these local files.

The Windows launcher imports each player's original class talent XP and ranks
from their own `%APPDATA%/wenilla-rogue/characters.txt` before starting the game.
It locates the installed original executable to read the save; portable installs
may need to select that executable. The original save is read-only. The current
talent book is backed up, higher local progress is preserved, and an unchanged
original save is not imported again. No personal saves are distributed in updates.

Other original character saves, collections and difficulty stars are not imported.
Heroic difficulties and later chapter unlocks
are not offered by the Elwynn-only build. This is not save-format compatibility
with `D:\Personal\ASR`.

## Boundaries

Co-op currently targets native desktop clients. The browser build remains solo.
Network interruption currently ends the affected connection; the original
launcher/relay's two-minute session resume has not been ported. There is no host
migration. Runs cannot be joined after they start. Rooms and in-flight matches
are not restored after a directory restart.

The relay enforces room membership, host/guest routing, capacity, build matching,
message sizes and request rates. Combat values and personal saves belong to
players, so this is a friends co-op model, not a trusted competitive economy.
Spell interactions and co-op gameplay still need player testing.
