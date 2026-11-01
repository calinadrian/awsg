# Ornate card assets

These original frame/background/diamond PNGs come from the approved Classic Ornate browser handoff (V16). Borders and diamonds have transparent backgrounds. Surfaces are flat with no glow. Frames use WoW quality colors: white, #1eff00, #0070dd, #a335ee and #ff8000.

Fonts are Cinzel regular/bold and Crimson Pro regular/semibold from the existing handoff. Game spell/item icons remain loaded at runtime from the player's own install; no reference screenshot or additional game artwork is bundled.

The renderer retains 420×640 coordinates with resolution-aware sizing. Body text uses Crimson Pro Semibold at 22 design units with a 26-unit line height. The hand aims for a minimum 13 logical screen pixels at lower resolutions, constrained by available width and reward-screen height; very small windows prioritize fitting the hand. Open cards refresh on window resize. NEW remains at top right for unowned spells and empty amulet slots. Muted current → next ranks sit below the title. Stat comparisons are supplied from real combat/reward parameters, with one row for each changed value. Amulet footers describe the actual occupied/empty slot.

The final readability proposal uses a 356×156 description area and a separate 372×116 stat panel. Rank text ends 16 design units above the separator. New spell cooldowns are mapped into the stat panel rather than appended to the prose. Up to three comparison rows retain 26-unit semibold values and 18-unit bold labels; longer values and larger row counts use smaller sizes.

Rank and stat arrows use the same transparent procedural image because the bundled text faces lack the arrow glyph.

Card text is re-shaped after the UI render scale has propagated, so resizing, maximizing and fullscreen transitions rebuild the font metrics at the same scale as their frame. UI scale is updated before dealing the hand.

Rarity labels use the bundled Cinzel Bold face at 14 design units. Separator rules use a brighter muted gold, 2.5 design units thick with a two-physical-pixel minimum after render scaling, and stay centred on the gem.
