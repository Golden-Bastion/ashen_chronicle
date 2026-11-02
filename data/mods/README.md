# Mod packages

Installed mods live in the game's private internal mod catalog. Each mod is installed in a directory named after its stable manifest ID. Distribute mods as ZIP archives and install them from the Options screen.

A ZIP may contain a root-level `manifest.json` or one top-level package directory containing the manifest and payload:

```text
example_mod.zip
└── example_mod/
    ├── manifest.json
    ├── content.json
    └── assets/
        └── locations/
            └── ruined_keep.png
```

The manifest identifies the package and may include campaign content:

```json
{
  "id": "example.mod",
  "name": "Example Mod",
  "version": "0.1.0",
  "priority": 0,
  "enabled": true,
  "content_file": "content.json"
}
```

The `content_file` field is optional. Omit it for an asset-only mod:

```json
{
  "id": "example.asset_replacement",
  "name": "Example Asset Replacement",
  "version": "1.0.0",
  "priority": 50,
  "enabled": true
}
```

A mod may add campaign content, provide assets only, or do both. Campaign content uses the same schema as the bundled base content. Cross-references may point to definitions supplied by the base game or other active mods; the full active content set is validated after merging.

## Mod-owned assets

All custom assets must belong to a mod. There is no separate writable global assets directory. A mod may add new artwork or replace a bundled base-game asset by providing a file with the same relative path.

```text
example_mod/
├── manifest.json
├── content.json
└── assets/
    ├── spatial/
    │   └── terrain_floor.png
    ├── icons/
    │   └── relic.png
    └── locations/
        └── ruined_keep.png
```

Bare relative asset references in base-game or mod content resolve as overlays: the highest-priority enabled mod providing that path wins. If priorities tie, the lexicographically higher stable mod ID wins, making the result deterministic. When no enabled mod provides the path, the bundled base asset is used.

To explicitly reference an asset from a particular mod, use its stable manifest ID:

```json
{
  "scene_image": "mod:example.mod/locations/ruined_keep.png"
}
```

This explicit form bypasses overlay precedence and resolves only inside the named enabled mod. Do not use the ZIP's outer directory name or the installed folder name as identity.

Only safe relative paths are accepted, and the current runtime supports PNG assets. Absolute paths, traversal, and symbolic links are rejected by the installer/resolver. Mod ZIPs are staged and validated before installation; malformed or unsafe archives must not replace an installed mod.

## Mod settings and load timing

Enabled state and priority are stored separately from the package manifest so the player's preferences are not overwritten by package contents. The Options screen lists installed mods and supports enabling/disabling, changing priority, removing, and ZIP installation.

Mod changes take effect on the next full game initialization. A running campaign keeps the content and asset set with which it started. Asset replacement conflicts and their winning providers can be inspected through the developer console's `content` diagnostics.

Saves record the active mod IDs, names, and versions. Loading warns about missing, disabled, or version-mismatched dependencies. The player may continue at their own responsibility. There is no automatic mod update or save migration system yet; compatibility remains the mod author's responsibility.
