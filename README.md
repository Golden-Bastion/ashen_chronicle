# The Ever-Unfolding Saga

A character-focused dark-fantasy RPG built in Rust, centered on exploration, self-directed lives, and adventures in a living world. The shipped game uses a Bevy graphical frontend while keeping gameplay and presentation models independent of the engine.

The game is inspired by the atmosphere and themes of dark fantasy: dangerous roads, forgotten places, strange characters, difficult choices, and a world that does not simply reset when a character dies.

## Features

- Procedurally generated world
- Character creation and inheritance
- Exploration and travel
- Threats and encounters
- Current turn-based combat implementation (planned direction: real-time spatial action combat)
- World reset and inheritance options on death
- Persistent world changes
- History/event tracking
- NPC memory, reactions, and faction reputation
- Quest system
- Base content loading and mod support foundation
- Bevy graphical user interface
- Keyboard interaction and clickable choices
- Optional ASCII portraits, item art, and location scenes

## Core Concept

The Ever-Unfolding Saga is a dark-fantasy open-world RPG about exploration, self-directed lives, and adventures shaped by each character.

Its procedurally generated world extends beyond the known frontier, offering new places, people, dangers, resources, and opportunities to discover. Exploration should reward curiosity without requiring every location to be handcrafted.

Players choose their own pursuits and shape their lives through the world. Hunting, exploration, relationships, settlement, trade, leadership, and other ways of living can become meaningful goals in their own right, rather than branches of a single predetermined storyline.

Death ends a character's life, but not necessarily their story. The world and its history persist, allowing a new character to inherit the consequences of previous lives and shape what happens next.

Handcrafted content and mods introduce distinctive characters, places, quests, events, and stories into a world whose possibilities extend beyond any single character's life.

The goal is to create a world worth exploring, a life worth shaping, and a history worth inheriting.
## Project Structure

```text
ashen_chronicle
├── data
│   ├── mods
│   │   ├── ashen_expansion
│   │   │   ├── content.json
│   │   │   └── manifest.json
│   │   ├── echoes_depth
│   │   │   ├── content.json
│   │   │   └── manifest.json
│   │   └── README.md
│   └── base_content.json
├── docs
│   ├── systems
│   │   ├── content.md
│   │   ├── events.md
│   │   ├── persistence.md
│   │   └── ui.md
│   ├── README.md
│   ├── design-principles.md
│   ├── architecture.md
│   ├── development-plan-history.md
│   └── roadmap-history.md
├── src
│   ├── content
│   │   ├── definitions.rs
│   │   ├── diagnostics.rs
│   │   ├── loader.rs
│   │   └── seeding.rs
│   ├── game
│   │   ├── actions.rs
│   │   ├── character.rs
│   │   ├── combat.rs
│   │   ├── console.rs
│   │   ├── console_commands.rs
│   │   ├── console_ui.rs
│   │   ├── history_screen.rs
│   │   ├── interactions.rs
│   │   ├── legacy.rs
│   │   ├── menu.rs
│   │   ├── navigation.rs
│   │   ├── quests.rs
│   │   ├── records.rs
│   │   ├── state_effects.rs
│   │   ├── time.rs
│   │   └── world.rs
│   ├── bevy_app.rs
│   ├── bevy_combat.rs
│   ├── bevy_console.rs
│   ├── bevy_gameplay.rs
│   ├── bevy_interactions.rs
│   ├── bevy_lifecycle.rs
│   ├── bevy_navigation_bridge.rs
│   ├── bevy_presentation.rs
│   ├── bevy_records.rs
│   ├── content.rs
│   ├── events.rs
│   ├── game.rs
│   ├── input.rs
│   ├── main.rs
│   ├── model.rs
│   ├── persistence.rs
│   ├── presentation.rs
│   ├── procedural.rs
│   ├── procedural_authored.rs
│   ├── procedural_characteristics.rs
│   ├── procedural_entities.rs
│   ├── procedural_opportunities.rs
│   ├── procedural_relationships.rs
│   ├── rng.rs
│   └── ui.rs
├── AGENTS.md
├── Cargo.lock
├── Cargo.toml
├── DEVELOPMENT_PLAN.md
├── LICENSE
├── README.md
└── ROADMAP.md
```

`main.rs` starts the Bevy application. `bevy_app.rs` configures the engine, while the other `bevy_*.rs` modules adapt lifecycle, gameplay, records, combat, console, and interaction flows to the shared presentation layer.

`game.rs` contains the gameplay module façade and core gameplay modules.

`model.rs` contains the core game data structures and world model.

`persistence.rs` handles saving and loading the world and preserves compatibility with older save formats.

`presentation.rs` contains frontend-independent view models, while `input.rs` contains frontend-neutral semantic interaction events.

## Saves

Save files are stored as gzip-compressed JSON using a character-specific filename such as `ashen_chronicle_save_Ash Walker.json.gz`. Existing `ashen_chronicle_save.json` saves from earlier versions remain readable.

## Building

The project uses Rust, Cargo, and Bevy.

Build the project with:

```sh
cargo build -r
```

Run the game with:

```sh
cargo run -r
```

or run the built release directly:

```sh
./ashen_chronicle
```

### Release targets

Official release builds target Android (x86_64, aarch64, armv7), Linux (x86_64, aarch64), macOS (aarch64 / Apple Silicon), and Windows (x86_64, aarch64). Intel macOS (x86_64-apple-darwin) is no longer an official release target. The intended platform scope is desktop plus Android; iOS is not a target. Single-player gameplay and local saves are intended to work fully offline, and updates should remain player-controlled where the distribution platform permits.

Run the test suite with:

```sh
cargo test
```

## Design Philosophy

#### The world should remember.
Important actions should have consequences that can survive beyond a single character.

#### Death should matter.
Character death is part of the game's progression rather than merely a failure state.

#### Systems should interact.
Quests, factions, NPCs, locations, combat, inventory, history, and world state should gradually become interconnected rather than existing as isolated mechanics.

#### Content should eventually be data-driven.
As the game grows, adding content should require less modification of the underlying engine.

#### Presentation should enhance the world.
Spatial gameplay is graphics-first: movement, exploration, atmosphere, and intended future action combat are presented spatially. Text remains essential for dialogue, descriptions, history, records, and contextual feedback. Preserve existing non-spatial actions where they fit; do not treat a text-first identity as a constraint on the intended game.

#### Keep the project maintainable.
Development should proceed incrementally, with the roadmap tracking delivered status and semantic versioning tracking releases. Current direction and implementation status are distinct; see [the canonical design principles](docs/design-principles.md).

#### Character experience comes first.
World simulation, economics, organisations, relationships, legacy, and procedural systems should create meaningful character experiences rather than turning the game into a remotely managed grand-strategy or maintenance simulator.

## Versioning

The project follows semantic versioning:

MAJOR.MINOR.PATCH

Development releases remain below `1.0.0` while the core systems and content are still being established.

## License

- Source Code: BUSL 1.1 (`LICENSE`) (converts to Apache License 2.0 on 2030-09-27)
- Game Content (everything under `data/`): CC BY-NC-ND 4.0 (`LICENSE-CONTENT`)
- Names and logos: `TRADEMARKS.md`. Third-party software: `THIRD-PARTY-NOTICES`.
- File-level licensing: `REUSE.toml`.

---

A world to explore, a life to shape, and adventures yet to unfold.
