mod db;
mod fs_ops;
mod github;
mod skills_sh;
mod validate;

use db::{InstallRecord, SearchRecord};
use fs_ops::{list_installed_skills, resolve_db_path, InstalledSkill};
use github::{install_github_candidate, search_github_repo, InstallGithubRequest, InstallGithubResult, SearchResult};
use skills_sh::SkillsSearchItem;
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────────────────
// Helper: open DB (returns a string error for Tauri command boundary)
// ──────────────────────────────────────────────────────────────────────────────
fn open_db() -> Result<rusqlite::Connection, String> {
    let db_path = resolve_db_path()?;
    db::open_db(&db_path).map_err(|e| format!("DB error: {}", e))
}

// ──────────────────────────────────────────────────────────────────────────────
// Commands
// ──────────────────────────────────────────────────────────────────────────────

/// Search a GitHub repo URL for SKILL.md candidates. Records the search in DB.
#[tauri::command]
fn search_github(url: String, agent: String) -> Result<SearchResult, String> {
    let result = search_github_repo(&url)?;

    // Record in DB (best-effort — don't fail the search if DB fails)
    if let Ok(conn) = open_db() {
        let _ = db::record_search(&conn, &agent, &url, &result.owner, &result.repo, &result.branch);
    }

    Ok(result)
}

/// Search skills.sh and return the first GitHub-backed catalog results.
#[tauri::command]
async fn search_skills(query: String) -> Result<Vec<SkillsSearchItem>, String> {
    skills_sh::search_skills(&query).await
}

/// Install one or more GitHub candidates in batch.
#[tauri::command]
fn install_github_skills(requests: Vec<InstallGithubRequest>, agent: String) -> Vec<InstallGithubResult> {
    let conn = open_db().ok();
    let mut results = Vec::new();

    for req in &requests {
        match install_github_candidate(req, &agent) {
            Ok(res) => {
                if let Some(ref conn) = conn {
                    let source_ref = format!(
                        "https://github.com/{}/{}/tree/{}/{}",
                        req.owner, req.repo, req.branch, req.candidate.dir_path
                    );
                    let _ = db::record_install_ok(
                        conn,
                        &agent,
                        &res.skill_name,
                        "github",
                        Some(&source_ref),
                        &res.installed_path,
                        res.files_written,
                    );
                }
                results.push(res);
            }
            Err(e) => {
                if let Some(ref conn) = conn {
                    let _ = db::record_install_error(
                        conn,
                        &agent,
                        &req.candidate.name,
                        "github",
                        None,
                        &e,
                    );
                }
                results.push(InstallGithubResult {
                    candidate_name: req.candidate.name.clone(),
                    skill_name: req.candidate.name.clone(),
                    original_name: req.candidate.name.clone(),
                    installed_path: String::new(),
                    files_written: 0,
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    results
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropInstallResult {
    pub path: String,
    pub skill_name: String,
    pub original_name: String,
    pub installed_path: String,
    pub files_written: i64,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
async fn install_dropped_paths(
    paths: Vec<String>,
    agent: String,
) -> Result<Vec<DropInstallResult>, String> {
    let conn = open_db().ok();
    let mut results = Vec::new();

    for p in paths {
        let raw_path = p.clone();
        let source_path = std::path::Path::new(&p);

        if !source_path.exists() {
            results.push(DropInstallResult {
                path: raw_path.clone(),
                skill_name: String::new(),
                original_name: String::new(),
                installed_path: String::new(),
                files_written: 0,
                success: false,
                error: Some("Dropped item does not exist or is not accessible.".to_string()),
            });
            continue;
        }

        match fs_ops::install_all_from_local_path(source_path, &agent) {
            Ok(installs) => {
                for res in installs {
                    if let Some(ref conn) = conn {
                        let _ = db::record_install_ok(
                            conn,
                            &agent,
                            &res.skill_name,
                            "upload",
                            Some(&raw_path),
                            &res.installed_path,
                            res.files_written,
                        );
                    }
                    results.push(DropInstallResult {
                        path: raw_path.clone(),
                        skill_name: res.skill_name,
                        original_name: res.original_name,
                        installed_path: res.installed_path,
                        files_written: res.files_written,
                        success: true,
                        error: None,
                    });
                }
            }
            Err(e) => {
                let name = source_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(ref conn) = conn {
                    let _ = db::record_install_error(conn, &agent, &name, "upload", Some(&raw_path), &e);
                }
                results.push(DropInstallResult {
                    path: raw_path.clone(),
                    skill_name: name,
                    original_name: String::new(),
                    installed_path: String::new(),
                    files_written: 0,
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    Ok(results)
}

/// List installed skills by reading the filesystem live, and augmenting with DB metadata.
#[tauri::command]
fn get_installed_skills(agent: String) -> Result<Vec<InstalledSkill>, String> {
    let mut skills = list_installed_skills(&agent)?;

    if let Ok(conn) = open_db() {
        if let Ok(sources) = db::get_install_sources(&conn) {
            for skill in &mut skills {
                if let Some(repo) = sources.get(&skill.name) {
                    skill.source_repo = Some(repo.clone());
                }
            }
        }
    }

    Ok(skills)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryData {
    pub searches: Vec<SearchRecord>,
    pub installs: Vec<InstallRecord>,
}

/// Fetch search + install history from SQLite.
#[tauri::command]
fn get_history() -> Result<HistoryData, String> {
    let conn = open_db()?;
    let searches = db::get_searches(&conn).map_err(|e| format!("DB error: {}", e))?;
    let installs = db::get_installs(&conn).map_err(|e| format!("DB error: {}", e))?;
    Ok(HistoryData { searches, installs })
}

/// Return the resolved skills directory path.
#[tauri::command]
fn get_skills_dir(agent: String) -> Result<String, String> {
    fs_ops::resolve_skills_dir(&agent).map(|p| p.to_string_lossy().to_string())
}

use tauri_plugin_opener::OpenerExt;

/// Open the skills directory natively
#[tauri::command]
fn open_skills_dir(app: tauri::AppHandle, agent: String) -> Result<(), String> {
    let dir = fs_ops::resolve_skills_dir(&agent)?;
    app.opener().open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("Failed to open folder: {}", e))
}

/// Delete a skill from the filesystem.
#[tauri::command]
fn delete_skill(agent: String, skill_name: String) -> Result<(), String> {
    fs_ops::delete_skill(&agent, &skill_name)
}

// ──────────────────────────────────────────────────────────────────────────────
// App entry point
// ──────────────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            search_github,
            search_skills,
            install_github_skills,
            install_dropped_paths,
            get_installed_skills,
            get_history,
            get_skills_dir,
            open_skills_dir,
            delete_skill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
