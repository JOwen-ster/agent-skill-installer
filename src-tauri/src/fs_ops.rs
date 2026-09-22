/// Filesystem operations: resolving the skills directory, copying skill files,
/// listing installed skills, and auto-renaming conflicts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::validate::validate_skill_md;

/// Resolve the skills directory according to the selected agent.
/// Creates the directory (and all parents) if it does not exist.
pub fn resolve_skills_dir(agent: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let base_folder = if agent == "claude" { ".claude" } else { ".codex" };
    
    let base = if let Ok(custom_home) = std::env::var(if agent == "claude" { "CLAUDE_HOME" } else { "CODEX_HOME" }) {
        PathBuf::from(custom_home)
    } else {
        home.join(base_folder)
    };
    
    let skills_dir = base.join("skills");
    std::fs::create_dir_all(&skills_dir)
        .map_err(|e| format!("Cannot create skills directory: {}", e))?;
    Ok(skills_dir)
}

/// Resolve the database directory path.
pub fn resolve_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join("agent-skill-installer.db"))
}

/// Find a unique name under `skills_dir` by appending -2, -3, etc. if needed.
/// Also enforces that the folder name uses dashes instead of spaces.
pub fn resolve_unique_name(skills_dir: &Path, desired_name: &str) -> String {
    // Replace all spaces with dashes
    let sanitized_name = desired_name.replace(' ', "-");
    
    let candidate = skills_dir.join(&sanitized_name);
    if !candidate.exists() {
        return sanitized_name;
    }
    let mut suffix = 2u32;
    loop {
        let name = format!("{}-{}", sanitized_name, suffix);
        if !skills_dir.join(&name).exists() {
            return name;
        }
        suffix += 1;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledSkill {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub source_repo: Option<String>,
}

/// List all installed skills by reading the filesystem live.
/// Each subdirectory that contains a SKILL.md is a skill.
pub fn list_installed_skills(agent: &str) -> Result<Vec<InstalledSkill>, String> {
    let skills_dir = resolve_skills_dir(agent)?;
    let mut skills = Vec::new();

    let entries = std::fs::read_dir(&skills_dir)
        .map_err(|e| format!("Cannot read skills directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skill_md_path = path.join("SKILL.md");
            if skill_md_path.exists() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                // Try to parse description from frontmatter
                let description = read_skill_description(&skill_md_path);
                skills.push(InstalledSkill {
                    name,
                    path: path.to_string_lossy().to_string(),
                    description,
                    source_repo: None,
                });
            }
        }
    }

    // Sort alphabetically
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

fn read_skill_description(skill_md_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_md_path).ok()?;
    let after_open = content.trim_start().strip_prefix("---")?;
    let after_open = after_open.trim_start_matches('\r').trim_start_matches('\n');
    let close_pos = after_open.find("\n---")
        .or_else(|| after_open.find("\r\n---"))?;
    let fm_str = &after_open[..close_pos];
    let yaml: serde_yaml::Value = serde_yaml::from_str(fm_str).ok()?;
    yaml.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallFromLocalResult {
    pub skill_name: String,
    pub installed_path: String,
    pub files_written: i64,
    pub original_name: String, // name before auto-rename
}

/// Find all valid skill files recursively under `dir`.
/// A file is considered a skill file if it ends with `.md` (case-insensitive) 
/// and passes `validate_skill_md`.
pub fn find_all_skills_in(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut skills = Vec::new();

    if dir.is_file() {
        if dir.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("md")).unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(dir) {
                if let Ok(parsed_name) = validate_skill_md(&content) {
                    skills.push((dir.to_path_buf(), parsed_name));
                }
            }
        }
        return skills;
    }

    for entry in WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            let path = entry.path();
            let is_md = path.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("md"))
                .unwrap_or(false);

            // Also check for exactly "skill.md" case-insensitive, or just any .md file.
            // The prompt says "search for all 'skill.md' files", but also "if a folder has many skill files in the same directory, create skill folders for each using the title". 
            // So we'll check any .md file.
            if is_md {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(parsed_name) = validate_skill_md(&content) {
                        skills.push((path.to_path_buf(), parsed_name));
                    }
                }
            }
        }
    }
    skills
}

/// Install all skills from a local path (drag-and-drop file or folder).
/// Returns a list of installation results.
pub fn install_all_from_local_path(source_path: &Path, agent: &str) -> Result<Vec<InstallFromLocalResult>, String> {
    let skills = find_all_skills_in(source_path);
    if skills.is_empty() {
        return Err(format!("No valid skill files found in {}", source_path.display()));
    }

    let skills_dir = resolve_skills_dir(agent)?;
    let mut results = Vec::new();

    for (md_path, parsed_name) in skills {
        let is_single_file = source_path.is_file();
        let skill_root = md_path.parent().unwrap_or(source_path);

        // Always use parsed_name to ensure each skill in the same directory gets its own unique folder
        let original_name = parsed_name.clone();
        
        let final_name = resolve_unique_name(&skills_dir, &original_name);
        let dest_dir = skills_dir.join(&final_name);

        // Copy to a temp directory first, then rename atomically
        let temp_dir = skills_dir.join(format!(".tmp-{}", final_name));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Cannot create temp dir: {}", e))?;

        let files_written;

        if is_single_file {
            files_written = match std::fs::copy(&md_path, temp_dir.join("SKILL.md")) {
                Ok(_) => 1,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(format!("Failed to copy SKILL.md: {}", e));
                }
            };
        } else {
            files_written = match copy_dir_all(skill_root, &temp_dir) {
                Ok(c) => c,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(e);
                }
            };

            // Ensure the specific skill file we are installing is named SKILL.md in the destination.
            // When copy_dir_all ran, it copied it exactly as it was named in skill_root.
            // Its relative path from skill_root is just its file name.
            if let Some(file_name) = md_path.file_name() {
                let dest_file_path = temp_dir.join(file_name);
                let skill_md_dest = temp_dir.join("SKILL.md");

                if dest_file_path != skill_md_dest {
                    if let Err(e) = std::fs::rename(&dest_file_path, &skill_md_dest) {
                        let _ = std::fs::remove_dir_all(&temp_dir);
                        return Err(format!("Failed to rename skill file to SKILL.md: {}", e));
                    }
                }
            }
        }

        std::fs::rename(&temp_dir, &dest_dir)
            .map_err(|e| {
                let _ = std::fs::remove_dir_all(&temp_dir);
                format!("Cannot move skill to destination: {}", e)
            })?;

        results.push(InstallFromLocalResult {
            skill_name: final_name,
            installed_path: dest_dir.to_string_lossy().to_string(),
            files_written,
            original_name,
        });
    }

    Ok(results)
}

/// Recursively copy `src` directory into `dst`, creating all necessary parent dirs.
/// Returns the number of files copied.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<i64, String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Cannot create destination directory: {}", e))?;

    let mut count = 0i64;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        let relative = entry.path().strip_prefix(src)
            .map_err(|e| format!("Path strip error: {}", e))?;
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|e| format!("Cannot create dir {}: {}", target.display(), e))?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {}", e))?;
            }
            std::fs::copy(entry.path(), &target)
                .map_err(|e| format!("Cannot copy file {}: {}", entry.path().display(), e))?;
            count += 1;
        }
    }
    Ok(count)
}

/// Delete a skill folder by its name.
pub fn delete_skill(agent: &str, skill_name: &str) -> Result<(), String> {
    let skills_dir = resolve_skills_dir(agent)?;
    let dest_dir = skills_dir.join(skill_name);
    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir)
            .map_err(|e| format!("Cannot delete skill folder {}: {}", skill_name, e))?;
    }
    Ok(())
}
