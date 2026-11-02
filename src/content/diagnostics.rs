use super::loader::load_campaign_content_report;
use std::fmt::Write;

pub fn campaign_content_load_diagnostics() -> String {
    let report = load_campaign_content_report();
    let paths = crate::game_paths::GamePaths::get();
    let installed = crate::mod_manager::list_installed_mods().unwrap_or_default();
    let conflicts = crate::mod_manager::asset_override_conflicts().unwrap_or_default();

    let mut output = String::new();
    let _ = writeln!(output, "internal game data: {}", paths.root.display());
    let _ = writeln!(output, "bundled assets: {}", paths.bundled_assets_dir.display());
    let _ = writeln!(output, "installed mods: {}", installed.len());
    for item in &installed {
        let _ = writeln!(
            output,
            "  {} | {} | version={} enabled={} priority={} assets={}",
            item.manifest.id, item.manifest.name, item.manifest.version,
            item.enabled, item.priority, item.asset_count
        );
    }
    let _ = writeln!(output, "active mods loaded: {}", report.loaded_mods.len());
    let _ = writeln!(output, "asset replacement conflicts: {}", conflicts.len());
    for conflict in conflicts {
        let providers = conflict.providers.iter()
            .map(|(id, priority)| format!("{id} (priority {priority})"))
            .collect::<Vec<_>>().join(", ");
        let _ = writeln!(output, "  {}: {} -> winner {}", conflict.path, providers, conflict.winner);
    }
    let _ = writeln!(output, "warnings: {}", report.warnings.len());
    for warning in &report.warnings {
        let _ = writeln!(output, "  {warning}");
    }
    let _ = writeln!(
        output,
        "loaded content: locations={} encounters={} events={} quests={} npcs={} factions={} items={}",
        report.content.world.locations.len(), report.content.encounters.len(),
        report.content.events.len(), report.content.quests.len(), report.content.npcs.len(),
        report.content.factions.len(), report.content.item_visuals.len()
    );
    output.trim_end().to_string()
}
