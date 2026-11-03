# Bundled UI icon assets

These PNGs are protected base-game resources packaged with the game. They provide the default graphical frontend icons. They are loaded at runtime through the mod-aware asset resolver.

A mod can replace an icon by including a PNG at the same relative path under its own `assets/` directory, for example `assets/icons/ui/back.png`. The enabled mod with the highest priority wins. Equal priorities are resolved deterministically by stable mod ID.

Use transparent PNG artwork with a 32×32 source canvas and nearest-neighbor sampling for a consistent pixel-art style. Mods may also provide additional icons and reference them with `mod:<manifest-id>/<relative-path>.png`.

Missing icons are logged and omitted; they are not replaced with Unicode glyphs or placeholder text.
