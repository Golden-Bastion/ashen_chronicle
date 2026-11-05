# The Ever-Unfolding Saga

## Development Plan

The Ever-Unfolding Saga is a character-focused dark-fantasy RPG built around exploration, relationships, personal ambitions, consequential choices, and a persistent world. The intended direction is graphics-first spatial gameplay, with text retained for dialogue, records, descriptions, history, and contextual feedback. The existing implementation may lag behind this direction; consult the roadmap for current delivery status.

## Development priorities

1. Make the character’s life, exploration, relationships, and adventures the primary experience.
2. Preserve the persistent-world model: a character can die without erasing the world, and legacy can continue where appropriate.
3. Build graphics-first spatial exploration and, over time, real-time spatial action combat without making every activity spatial or replacing useful existing systems indiscriminately.
4. Keep gameplay systems data-driven and world simulation independent from presentation.
5. Preserve save compatibility and world-state continuity as systems evolve.
6. Keep system boundaries explicit and build/test incrementally instead of introducing large coupled features.

The approved direction is recorded in [the canonical design principles](docs/design-principles.md). They supersede older text-first assumptions where the two conflict.

## Current architecture direction

The codebase is being decomposed by responsibility. Runtime orchestration, gameplay actions, combat, screens, world/bootstrap logic, presentation, content loading, events, persistence, and UI should remain independently understandable.

See [`docs/architecture.md`](docs/architecture.md) for the architectural model and [`docs/systems/`](docs/systems/) for system-specific documentation.

## Development approach

Gameplay content should remain external to core logic where practical. Content must have stable IDs, schemas, validation, and explicit references. World mutations should go through clear ownership boundaries, and important state changes should be persisted and represented in world history where appropriate.

New systems should include focused tests for the behavior they introduce or modify. Refactors should preserve existing gameplay, save compatibility, and screen flow unless a change explicitly targets those areas.

## Mod/content progression

Mod support should evolve incrementally:

- Base campaign content loaded from structured data.
- External mod discovery and validation.
- Stable-ID-based replacement and extension.
- More advanced scripted hooks only after the data-driven foundation is stable.

## Long-term goals

The project should grow toward a deeper, graphics-first spatial RPG with a persistent simulation beneath the character-centred experience—not a text-first game with graphics merely added as decoration, and not a full grand-strategy or maintenance simulator. Procedural generation, quests, events, progression, memory, relationships, legacy, audiovisual feedback, and mod support should build on clear ownership and data-driven foundations. Real-time spatial combat is a future design direction; the current turn-based implementation remains the compatibility baseline until a deliberate migration.

## Working rules

- Treat [docs/design-principles.md](docs/design-principles.md) as the product-direction authority when older prose conflicts.
- Clearly distinguish desired direction from implemented capability and platform validation.
- Core single-player gameplay and local saves should work offline; network services must remain optional.
- Player choice should govern update installation wherever the delivery platform permits.
- Every entity has a stable unique ID.
- References use IDs rather than display names.
- Systems own their state and expose explicit mutation paths.
- Content is validated before entering runtime state.
- Avoid splitting files purely to reduce line counts; split at responsibility boundaries.
- Preserve compatibility where practical, and add focused tests when behavior changes.

## Historical plan

The original development plan is preserved in [`docs/development-plan-history.md`](docs/development-plan-history.md).
