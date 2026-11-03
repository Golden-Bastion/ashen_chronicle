# UI System

The shipped frontend is Bevy. The UI layer is built around renderer-neutral presentation views and semantic interaction events so gameplay systems do not depend on GUI details. The product direction is graphics-first spatial gameplay: spatial views carry movement, exploration, environmental context, and the intended future action-combat experience. Text remains important for dialogue, descriptions, records, history, and contextual feedback, but is not the overriding presentation model. See [the canonical design principles](../design-principles.md).

## Screen architecture

The Bevy frontend uses dedicated screen flows for start, save selection, character creation, gameplay, navigation, the unified pause/navigation menu, Options, NPC dialogue, remains recovery, records, combat, developer console, quit, and death. `NavigationState` tracks the active screen and return destination so nested views can return to the correct parent.

Lifecycle screens are owned by `bevy_lifecycle.rs`. Gameplay and world navigation are owned by `bevy_gameplay.rs`. NPC dialogue and remains recovery are owned by `bevy_interactions.rs`. Character, inventory, quest, meditation, history, and journal flows are owned by `bevy_records.rs`. Combat is owned by `bevy_combat.rs`, and the developer console is owned by `bevy_console.rs`.

## Presentation boundary

`presentation.rs` contains frontend-independent view models. These models carry domain data such as strings, numbers, identifiers, and collections without depending on Bevy or any terminal UI library.

Bevy presentation adapters turn those views into UI nodes. Shared node construction, semantic visual theme tokens, screen roots, surfaces, overlays, labels, action/menu controls, health gauges, condition indicators, contextual messages, navigation state, and semantic input routing live in `bevy_presentation.rs`.

The v0.51 graphical foundation keeps legacy panel helpers for existing screens while adding meaning-oriented primitives for the redesigned interface. New action and menu controls expose explicit interaction-state styling, health gauges keep their display values synchronized with their component state, and responsive spacing/touch-target tokens are centralized so new screens do not invent per-screen sizing rules. The dedicated Options screen reuses these primitives rather than introducing a separate menu architecture. Options provides Saves, Mods, and Interface tabs on both platforms. Interface settings expose a shared UI scale and font-size preference, persisted in the application-managed configuration directory. Desktop and Android launch fullscreen; Android also retains its immersive system-UI hiding and landscape activity orientation.

## Input

Native Bevy keyboard, touch, and button interaction is translated into the semantic `InputEvent` model in `input.rs`. Gameplay and screen systems consume these semantic events rather than Bevy-specific keyboard types except where text entry requires native `KeyboardInput` text data.

Android touch positions are logical window coordinates while Bevy UI layout hit-testing uses physical coordinates, so the shared touch handling converts touch positions with the window scale factor before testing UI bounds. This conversion is shared by custom choice and scroll hit-testing.

Lifecycle character-creation fields use Bevy 0.19.1's `EditableText` with real `InputFocus`, tab navigation, cursor/edit handling, and native IME routing. The shared presentation layer still gives the fields explicit touch targets so touch input can assign native focus. On Android, the GameActivity text-input state is also synchronized with the focused field so character commits remain available even when the Bevy IME event stream exposes only key events. The custom IME-to-keyboard bridge remains only for the developer console, whose input model is still renderer-specific. Android IME dismissal clears native lifecycle focus before semantic Back navigation resumes.

Arrow keys, Home/End, Page Up/Page Down, Enter, Escape, Tab, Backspace, Delete, and selected number/vim-style shortcuts remain available where each screen supports them. UI buttons produce the same semantic confirmation events as keyboard input.

## Gameplay and results

The gameplay surface is world-first: location art/context occupies the dominant area, the 12-point world clock drives an obvious sun/moon sky treatment, and atmosphere can move into a short-lived contextual message. A compact top-right HUD carries health and active-condition indicators beside the secondary-navigation control. Primary actions remain below the world presentation, with contextual Investigate/Search Remains actions added only when the authoritative game state makes them available. Recent history is no longer a permanent gameplay panel; full history remains in its dedicated record screen. World navigation is a dedicated state within the gameplay flow.

NPC dialogue presents people at the current location, availability, faction and memory information, quest offering/turn-in choices, and conversation results. Remains recovery presents available corpses, recovered items, discovered hidden items, and recovery notes while keeping the authoritative corpse mutation in the game legacy module.

Combat presents player/enemy status, encounter events, action choices, and result details through Bevy nodes while the authoritative combat system continues to own resolution and state mutation.

Record screens present character data, inventory details, quests, meditation choices/results, history entries/details, and journal editing. Results stay on-screen until the user advances or returns to the parent flow.

## Developer console

The developer console uses renderer-neutral `ConsoleView` data. Command editing, history navigation, completion candidates, scrolling, and command execution state remain in the game console modules; `bevy_console.rs` is responsible for graphical presentation and input handling.

The console opens from the gameplay flow and closes back to gameplay without a terminal-mode transition. Desktop users can open it with the `/` shortcut; touchscreen users can open it by long-pressing the gameplay Pause button for 0.65 seconds. A normal short tap continues to open the pause/navigation menu, while dragging cancels the long-press gesture.

## Visual design

The shared Bevy presentation layer uses a restrained dark theme with semantic surface, border, hover, pressed, selected, disabled, and overlay tokens. Legacy panels remain available for existing screens, while `spawn_surface` and `spawn_overlay` provide the non-dashboard structural primitives used by the v0.51 graphical redesign.

Meaning-oriented helpers include compact icon+text action buttons, compact menu buttons, health gauges with overlaid current/max values, condition indicators, and contextual messages. Action icons use the muted grayscale palette, while health gauges use a blood fill with a darker offset shadow layer. Shared touch targets retain a 48px baseline minimum before user-selected UI scaling. The Interface options tab provides independent persisted UI-scale and font-size controls on desktop and Android. UI scale adjusts fixed-size UI values and rendered text; the root font size remains viewport-responsive and is multiplied by the separate font-size preference.

The unified gameplay pause/navigation menu is a centered overlay with a soft dimming layer so the underlying gameplay remains visible. It contains Resume, the dedicated Character/Inventory/Quests/History/Journal screens, New Game, conditional Load Game, Options, and Quit. Opening a child screen from the menu preserves the paused-menu state so normal Back navigation returns to the same menu rather than dropping to the dashboard. Android system Back and desktop Escape both use the semantic Cancel path; from the gameplay dashboard they open this unified menu. The menu surface is scrollable so its full action set remains usable on smaller touch displays. Entering the menu also sets the explicit GameplayRuntimeState pause flag; continuously running gameplay systems must check that state, while UI/input handling continues normally.

Text remains a first-class presentation tool for dialogue, description, logs, records, and accessible contextual feedback. Within spatial gameplay, however, the world and player action should be communicated visually and spatially; artwork is not merely optional decoration for an otherwise text-first experience. Text and other fallbacks should keep content usable when assets are missing, without defining the intended overall presentation.

## Target visual composition (concept direction)

The intended interface is a dark, restrained frame around an expressive pixel-art world: near-black/charcoal surfaces, fine borders, warm brass/gold accents, compact iconography, and clear semantic colours for status. Let the environment dominate spatial gameplay rather than treating the world as a background to a dashboard. Use contrast, silhouettes, layered depth, lighting, and animation to keep people, threats, routes, and interactable objects readable.

The following layouts interpret the approved visual reference. They describe the intended information hierarchy, not a claim that every element already exists in the current build. Exact labels, numbers, icons, and key bindings may change as systems and usability tests mature.

### Exploration HUD

- Keep essential character status visible without covering the view: a portrait or clear character marker with health and other currently relevant vitals.
- Place current objective/quest guidance in a compact, dismissible area, and show a contextual interaction prompt near the person, object, exit, or point of interest it refers to.
- Make local orientation and time context available with a compact compass/minimap and a readable time-of-day cue where useful.
- Keep secondary shortcuts compact and contextual. Avoid permanent panels for systems that are not relevant to the current activity.
- Use in-world markers and visual feedback to distinguish nearby opportunities from background scenery without turning every object into a labelled icon.

### Real-time combat HUD

- Preserve the spatial view and make combat state legible at a glance: target identity and health, player vitals, available abilities, and important resource/cooldown feedback.
- Communicate enemy attacks and dangerous areas through clear spatial telegraphs, not text alone.
- Show nearby allies and their current state when that information affects a decision. Offer a small set of plausible local commands and access to slow-motion tactical orders; do not present a remote army-command interface.
- Let the combat HUD add detail to the exploration HUD when needed, then recede when the encounter ends.

### Character hub, inventory, and equipment

- Treat the character screen as a navigable hub rather than one oversized panel. Group overview, attributes, conditions, skills, relationships, journal/quests, reputation, and legacy into clear sections or tabs where the underlying systems support them.
- Make identity and progression easy to scan: portrait, name/background, level or milestones, relevant attributes, conditions, and traits.
- Show equipped gear in relation to the character, with identifiable slots and an item grid or list that can be filtered by category. Communicate item properties and carrying limits where they matter.
- Keep equipment, inventory, skills, and records closely connected without forcing every detail onto the spatial HUD.

### World map and travel

- Use a readable regional map to present known locations, routes or connections, points of interest, and concise location context. Make travel a deliberate action from this view.
- Let map annotations reflect what the character knows. A location summary may surface services, danger, or useful opportunities when that information is available.
- The map supports travel between local spatial areas; it is not a replacement for spatial exploration or a remote grand-strategy control surface.

### Shared feedback and mobile adaptation

- Reuse consistent visual patterns for buttons, status/resource bars, icons, item pickup and quest notifications, dialogue, and contextual messages. Use colour alongside iconography or text so meaning is not communicated by colour alone.
- On Android, preserve the same gameplay priorities while reflowing and scaling the HUD for a smaller screen. Provide reachable touch controls for movement and primary actions, with larger contextual/combat controls placed where thumbs can comfortably reach them. The virtual joystick and action buttons should not obscure the character, nearby interactions, or important combat telegraphs.
- On desktop, keep keyboard/mouse guidance available where useful; the reference's WASD, interaction key, ability numbers, and character-menu key are examples, not immutable bindings. Controls should remain discoverable and remappable where supported.
- Adapt composition per platform rather than shrinking a desktop screenshot into a phone viewport. Maintain readable text, clear hit targets, and the ability to see the world while acting.

### Spatial exploration vitals

Spatial exploration now keeps a compact character panel in the header with character name/level, current and maximum health, and a text summary of active conditions. It reuses the common health-gauge primitive and shared dark theme, so the essential status remains visible without occupying a permanent large dashboard. This does not imply that the planned combat HUD, minimap, or ally command controls exist yet.

## v0.51 graphical foundation

Issue #278 establishes the reusable presentation primitives for the redesigned gameplay HUD and menu surfaces. Issue #279 applies those primitives to the gameplay surface with a world-first composition, responsive primary/contextual actions, dynamic time-of-day sky treatment, and the compact player HUD. Issue #317 unifies the former secondary-navigation and pause flows into the gameplay pause/navigation menu, while #227 supplies the dedicated tabbed Options screen.

## Design direction

Keep screen responsibilities separate from gameplay mechanics. New UI work should reuse frontend-neutral view models and semantic input events rather than introducing renderer-specific dependencies into gameplay rules. Adapt layouts and controls to desktop and mobile, supporting touch, mouse/keyboard, and controllers where practical. Keep spatial gameplay uncluttered, use contextual actions for nearby interactions, and reserve detailed management/information for hierarchical record screens. Avoid maintaining parallel terminal and graphical implementations or undertaking a wholesale UI redesign without a specific need.
