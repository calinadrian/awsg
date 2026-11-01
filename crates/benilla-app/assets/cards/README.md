# Arcane Codex card assets

Supplied in the user's `Arcane-Cards-Final-Handoff/arcane-card-handoff` V6 handoff.
`card_view.rs` embeds these files so a packaged game needs no separate asset folder.

- `background-*`, `border-*`, `diamond-*`: unchanged transparent PNG exports.
- `font-0.ttf` / `font-1.ttf`: Cinzel regular / bold.
- `font-2.ttf` / `font-3.ttf`: Crimson Pro regular / semibold.
  The source names ended in `.woff2`, but their bytes are TrueType; only the extensions changed.
- `preview-imgSpellArtwork*`: original handoff examples, used only by the isolated
  Arcane preview. Game cards continue to resolve their real spell/equipment icons.

The native card coordinates follow the 420x640 ornate format, including the 64px
top/bottom inset. The game displays them at 72% on its 768-unit UI canvas. The
flat surfaces, rarity colors, NEW/RANK badges, fonts and separate transparent
frames follow V6. Spell values and upgrade deltas come from the live reward data.
