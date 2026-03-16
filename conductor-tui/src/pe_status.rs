//! PE Status Panel — reads extraction-roadmap filesystem state
//!
//! Displays:
//! - Active cycles (discover-cycle-N, extract-cycle-N, etc.)
//! - Task status summary per cycle
//! - Alignment scores from latest ALIGNMENT_CHECK
//! - Blocked tasks requiring human input

use std::path::Path;

/// PE extraction roadmap root
const PE_ROADMAP_PATH: &str = "/usr/local/bsg/pattern-extractor/extraction-roadmap";

/// Summary of a single PE cycle
#[derive(Debug, Clone)]
pub struct PeCycleSummary {
    pub source: String,
    pub cycle_type: String,
    pub cycle_number: u32,
    pub status: String,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub blocked_tasks: u32,
}

/// Scan the extraction-roadmap directory for active cycles
pub fn scan_pe_cycles() -> Vec<PeCycleSummary> {
    let roadmap = Path::new(PE_ROADMAP_PATH);
    if !roadmap.is_dir() {
        return Vec::new();
    }

    let mut summaries = Vec::new();

    // Iterate source directories
    if let Ok(sources) = std::fs::read_dir(roadmap) {
        for source_entry in sources.flatten() {
            let source_path = source_entry.path();
            if !source_path.is_dir() {
                continue;
            }
            let source_name = source_entry.file_name().to_string_lossy().to_string();

            // Look for cycle directories
            if let Ok(cycles) = std::fs::read_dir(&source_path) {
                for cycle_entry in cycles.flatten() {
                    let cycle_name = cycle_entry.file_name().to_string_lossy().to_string();
                    // Parse cycle type and number from name like "discover-cycle-1"
                    if let Some((cycle_type, cycle_num)) = parse_cycle_name(&cycle_name) {
                        let (total, completed, blocked) =
                            count_tasks(&cycle_entry.path());
                        summaries.push(PeCycleSummary {
                            source: source_name.clone(),
                            cycle_type,
                            cycle_number: cycle_num,
                            status: infer_cycle_status(total, completed, blocked),
                            total_tasks: total,
                            completed_tasks: completed,
                            blocked_tasks: blocked,
                        });
                    }
                }
            }
        }
    }

    // Sort by source, then cycle type, then number
    summaries.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then(a.cycle_type.cmp(&b.cycle_type))
            .then(a.cycle_number.cmp(&b.cycle_number))
    });

    summaries
}

fn parse_cycle_name(name: &str) -> Option<(String, u32)> {
    // Parse "discover-cycle-1" -> ("discover", 1)
    // Split on "-cycle-" to handle multi-word cycle types
    let parts: Vec<&str> = name.splitn(2, "-cycle-").collect();
    if parts.len() == 2 {
        if let Ok(num) = parts[1].parse::<u32>() {
            return Some((parts[0].to_string(), num));
        }
    }
    None
}

/// Count task files in a cycle directory by scanning for status markers
fn count_tasks(cycle_path: &Path) -> (u32, u32, u32) {
    let mut total = 0u32;
    let mut completed = 0u32;
    let mut blocked = 0u32;

    // Look for task directories or task files
    if let Ok(entries) = std::fs::read_dir(cycle_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Task directories are typically named "task-NNN" or similar
            if name.starts_with("task-") || name.starts_with("phase-") {
                total += 1;
                // Check for status file inside task dir
                let status_file = entry.path().join("status");
                if let Ok(status) = std::fs::read_to_string(&status_file) {
                    let status = status.trim().to_lowercase();
                    if status == "complete" || status == "completed" || status == "done" {
                        completed += 1;
                    } else if status == "blocked" {
                        blocked += 1;
                    }
                }
            }
        }
    }

    (total, completed, blocked)
}

fn infer_cycle_status(total: u32, completed: u32, blocked: u32) -> String {
    if total == 0 {
        "empty".to_string()
    } else if completed == total {
        "complete".to_string()
    } else if blocked > 0 {
        "blocked".to_string()
    } else {
        "active".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cycle_name() {
        assert_eq!(
            parse_cycle_name("discover-cycle-1"),
            Some(("discover".to_string(), 1))
        );
        assert_eq!(
            parse_cycle_name("extract-cycle-3"),
            Some(("extract".to_string(), 3))
        );
        assert_eq!(
            parse_cycle_name("operate-cycle-12"),
            Some(("operate".to_string(), 12))
        );
        assert_eq!(parse_cycle_name("not-a-cycle"), None);
        assert_eq!(parse_cycle_name("campaigns"), None);
    }

    #[test]
    fn test_infer_cycle_status() {
        assert_eq!(infer_cycle_status(0, 0, 0), "empty");
        assert_eq!(infer_cycle_status(5, 5, 0), "complete");
        assert_eq!(infer_cycle_status(5, 2, 1), "blocked");
        assert_eq!(infer_cycle_status(5, 2, 0), "active");
    }
}
