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
and gold bonuses. Meals heal the player collecting them. Arena caches and
boss/guardian rewards give each player their own choices. In co-op, a card,
item choice or pause menu removes only that player from enemy targeting and
protects them from damage while the encounter and other players continue.
Their automatic attacks wait until they return. Loading screens still hold
the encounter until party members are ready; network traffic uses real time.
Solo reward screens continue to pause the run.
Fallen players watch until the party wins or everyone falls. A quitting or
crashing host ends the party.

Party frames show names, levels, health and choosing/loading status beneath
the player frame. Remote heroes also have overhead names and health bars.
Remote enemy and player movement is interpolated between 10 Hz snapshots;
clients use TCP_NODELAY, and snapshot/hit lookups use entity-ID indexes.
These address known delay and repeated-scan costs; actual latency and frame
rate on players' machines still need gameplay confirmation.

## Personal progress

Each PC keeps its class XP and talents in `survivors-talents.txt`, using the same
payout path for solo and co-op. `survivors-progress.toml` keeps per-class,
per-zone runs, victories, co-op victories, best survival time and best kills.
Rewards settle once when results open or the player returns to the menu.
Neither the relay nor another player's host can replace these local files.

The recreation does not import the original binary's character saves,
collections or difficulty stars. Heroic difficulties and later chapter unlocks
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
