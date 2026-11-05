# Documentation

This directory contains the project's detailed technical and historical documentation.

## Structure

- [`design-principles.md`](design-principles.md) — canonical product direction across gameplay, simulation, interface, modding, technical foundations, accessibility, and platforms.
- [`architecture.md`](architecture.md) — overall codebase architecture, module boundaries, responsibilities, and technical design rules.
- [`systems/`](systems/) — documentation for individual game systems.
- [`roadmap-history.md`](roadmap-history.md) — historical record of completed roadmap milestones.
- [`itchio-description.html`](itchio-description.html) — current long-form itch.io page description, ready to paste into the page editor.
- [`itchio-publishing.md`](itchio-publishing.md) — GitHub Actions and Butler release-publishing setup.
- [`development-plan-history.md`](development-plan-history.md) — historical record of the original development plan.

## Top-level documents

[`../ROADMAP.md`](../ROADMAP.md) contains the current and upcoming roadmap.

[`../DEVELOPMENT_PLAN.md`](../DEVELOPMENT_PLAN.md) contains the current development strategy and implementation priorities.

## Documentation rules

Keep current product direction in [`design-principles.md`](design-principles.md); implementation strategy in the top-level documents; and technical guidance in `docs/architecture.md` / `docs/systems/`. When old wording conflicts with the design principles, update it rather than treating historical assumptions as binding.

Historical documents are preserved for reference and should generally not be edited retroactively.

System documentation should describe how a system works and its boundaries, rather than becoming a chronological development log.'

