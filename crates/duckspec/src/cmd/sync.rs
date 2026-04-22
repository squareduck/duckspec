use std::collections::{HashMap, HashSet};
use std::path::Path;

use duckpond::artifact::spec::{Backlink, TestMarkerKind};
use duckpond::backlink::SourceBacklink;
use duckpond::config::Config;
use duckpond::layout::{self, ArtifactKind};
use duckpond::parse::{self, Span};
use ignore::WalkBuilder;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root};

pub fn run(dry: bool) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;
    let project_root = duckspec_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("duckspec/ has no parent directory"))?;
    let config = Config::load(&duckspec_root).map_err(|e| anyhow::anyhow!("{e}"))?;

    // 1. Scan source files for @spec backlinks.
    eprintln!("  {} scanning for backlinks…", "·".dimmed());
    let backlinks = scan_source_files(project_root, &duckspec_root, &config)?;

    // 2. Group backlinks by (cap_path, requirement, scenario).
    let grouped = group_backlinks(&backlinks, project_root);

    // 3. For each cap spec, update test:code markers with resolved paths.
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() {
        eprintln!("  {} no caps/ directory", "·".dimmed());
        return Ok(());
    }

    let spec_files = collect_files(&caps_dir)?;
    let mut changed_count = 0usize;

    for file_path in &spec_files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(&canonical_root).ok() else {
            continue;
        };
        if layout::classify(relative) != Some(ArtifactKind::CapSpec) {
            continue;
        }

        let cap_path = extract_cap_path(relative);
        let source = std::fs::read_to_string(file_path)?;
        let elements = parse::parse_elements(&source);
        let Ok(mut spec) = parse::spec::parse_spec(&elements) else {
            continue;
        };

        // Update backlinks in the parsed spec.
        let mut modified = false;
        for req in &mut spec.requirements {
            for scn in &mut req.scenarios {
                let key = BacklinkKey {
                    cap_path: cap_path.clone(),
                    requirement: req.name.clone(),
                    scenario: scn.name.clone(),
                };

                let is_test_code = scn
                    .test_marker
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. }))
                    || req
                        .test_marker
                        .as_ref()
                        .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. }));

                if !is_test_code {
                    continue;
                }

                let new_links: Vec<Backlink> = grouped
                    .get(&key)
                    .map(|paths| {
                        let mut sorted: Vec<_> = paths.iter().collect();
                        sorted.sort();
                        sorted
                            .into_iter()
                            .map(|p| Backlink {
                                path: p.clone(),
                                span: Span {
                                    offset: 0,
                                    length: 0,
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Ensure the scenario has its own test marker (not just inherited).
                let marker =
                    scn.test_marker
                        .get_or_insert_with(|| duckpond::artifact::spec::TestMarker {
                            kind: TestMarkerKind::Code {
                                backlinks: Vec::new(),
                            },
                            span: Span {
                                offset: 0,
                                length: 0,
                            },
                        });

                if let TestMarkerKind::Code { backlinks } = &mut marker.kind {
                    let old_paths: Vec<&str> = backlinks.iter().map(|b| b.path.as_str()).collect();
                    let new_paths: Vec<&str> = new_links.iter().map(|b| b.path.as_str()).collect();

                    if old_paths != new_paths {
                        *backlinks = new_links;
                        modified = true;
                    }
                }
            }
        }

        if !modified {
            continue;
        }

        let rendered = spec.render();
        if rendered == source {
            continue;
        }

        changed_count += 1;

        if dry {
            eprintln!("  {} {}", "would update".yellow(), relative.display());
        } else {
            std::fs::write(file_path, &rendered)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", file_path.display()))?;
            eprintln!("  {} {}", "updated".green(), relative.display());
        }
    }

    if changed_count == 0 {
        eprintln!("  {} no changes", "·".dimmed());
    } else if dry {
        eprintln!(
            "  {} {} file(s) would be updated",
            "·".dimmed(),
            changed_count
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Backlink grouping
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BacklinkKey {
    cap_path: String,
    requirement: String,
    scenario: String,
}

/// Group source backlinks by scenario, formatting paths as "relative/path:line".
fn group_backlinks(
    backlinks: &[SourceBacklink],
    project_root: &Path,
) -> HashMap<BacklinkKey, HashSet<String>> {
    let mut map: HashMap<BacklinkKey, HashSet<String>> = HashMap::new();
    for bl in backlinks {
        let key = BacklinkKey {
            cap_path: bl.cap_path.clone(),
            requirement: bl.requirement.clone(),
            scenario: bl.scenario.clone(),
        };
        let rel_path = bl.file.strip_prefix(project_root).unwrap_or(&bl.file);
        let link_str = format!("{}:{}", rel_path.display(), bl.line);
        map.entry(key).or_default().insert(link_str);
    }
    map
}

/// Extract cap path from a relative spec path.
/// e.g. "caps/auth/oauth/spec.md" → "auth/oauth"
fn extract_cap_path(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if components.len() >= 3 {
        components[1..components.len() - 1].join("/")
    } else {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Source scanning (shared logic with audit)
// ---------------------------------------------------------------------------

fn scan_source_files(
    project_root: &Path,
    duckspec_root: &Path,
    config: &Config,
) -> anyhow::Result<Vec<SourceBacklink>> {
    let scan_roots = if config.test_paths.is_empty() {
        vec![project_root.to_path_buf()]
    } else {
        config
            .test_paths
            .iter()
            .map(|p| project_root.join(p))
            .filter(|p| p.exists())
            .collect()
    };

    let duckspec_canonical = duckspec_root.canonicalize()?;
    let mut all_backlinks = Vec::new();

    for root in &scan_roots {
        let walker = WalkBuilder::new(root).build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Ok(canonical) = path.canonicalize()
                && canonical.starts_with(&duckspec_canonical)
            {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let found = duckpond::backlink::scan_file(path, &content);
            all_backlinks.extend(found);
        }
    }

    Ok(all_backlinks)
}
