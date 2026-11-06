# Celestial world art

`sun-moon.png` is a transparent landscape artwork atlas used by the gameplay world clock. The sun occupies the left panel and the moon occupies the right panel. The renderer crops each panel into a large, responsive sky sprite; neither body is a UI icon.

The bundled image is loaded through the mod-aware asset resolver. A mod can replace it by providing `assets/world/celestial/sun-moon.png` in its package. Keep both subjects in their existing left/right regions and preserve transparent space around their artwork so the sky background remains visible.
