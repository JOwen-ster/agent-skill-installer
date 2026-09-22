/// GitHub API integration: search for SKILL.md candidates in a repo,
/// download and install selected candidates.

use serde::{Deserialize, Serialize};

use crate::fs_ops::{resolve_skills_dir, resolve_unique_name};
use crate::validate::validate_skill_md;

/// Parsed representation of a GitHub URL.
#[derive(Debug, Clone)]
pub struct GitHubRef {
    pub owner: String,
    pub repo: String,
    pub branch: Option<String>,
}

/// Parse a GitHub URL into owner/repo/optional-branch.
/// Accepts:
///   https://github.com/owner/repo
///   https://github.com/owner/repo/tree/branch
///   owner/repo
///   owner/repo@branch
pub fn parse_github_url(url: &str) -> Result<GitHubRef, String> {
    let url = url.trim();

    // Full URL
    if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
        let path = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("github.com/");
        let parts: Vec<&str> = path.splitn(5, '/').collect();
        if parts.len() < 2 {
            return Err("Invalid GitHub URL: expected owner/repo".to_string());
        }
        let owner = parts[0].to_string();
        let repo = parts[1].trim_end_matches(".git").to_string();
        let branch = if parts.len() >= 4 && parts[2] == "tree" {
            Some(parts[3].to_string())
        } else {
            None
        };
        return Ok(GitHubRef { owner, repo, branch });
    }

    // shorthand owner/repo[@branch]
    let (path_part, branch) = if let Some((left, right)) = url.split_once('@') {
        (left, Some(right.to_string()))
    } else {
        (url, None)
    };

    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() < 2 {
        return Err(format!("Cannot parse as a GitHub repo reference: '{}'", url));
    }
    Ok(GitHubRef {
        owner: parts[0].to_string(),
        repo: parts[1].trim_end_matches(".git").to_string(),
        branch,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillCandidate {
    /// Directory path within the repo (relative, e.g. "skills/pdf-tools")
    pub dir_path: String,
    /// The skill folder name (last component of dir_path)
    pub name: String,
    /// Path to the SKILL.md within the repo
    pub skill_md_path: String,
    /// All file paths in that directory
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub candidates: Vec<SkillCandidate>,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubTreeItem>,
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

#[derive(Debug, Deserialize)]
struct RepoInfo {
    default_branch: String,
}

/// Search a GitHub repo for SKILL.md candidates.
pub fn search_github_repo(url: &str) -> Result<SearchResult, String> {
    let gh_ref = parse_github_url(url)?;
    let client = build_client()?;

    // Resolve branch
    let branch = match gh_ref.branch {
        Some(b) => b,
        None => {
            let repo_url = format!(
                "https://api.github.com/repos/{}/{}",
                gh_ref.owner, gh_ref.repo
            );
            let resp = client
                .get(&repo_url)
                .send()
                .map_err(|e| format!("Network error fetching repo info: {}", e))?;

            if resp.status() == 403 {
                return Err("GitHub rate limit reached (60/hr unauthenticated) — try again later.".to_string());
            }
            if resp.status() == 404 {
                return Err(format!("Repository {}/{} not found.", gh_ref.owner, gh_ref.repo));
            }
            if !resp.status().is_success() {
                return Err(format!("GitHub API error: {} {}", resp.status(), resp.status().canonical_reason().unwrap_or("")));
            }

            let info: RepoInfo = resp.json()
                .map_err(|e| format!("Failed to parse repo info: {}", e))?;
            info.default_branch
        }
    };

    // Fetch the full tree
    let tree_url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        gh_ref.owner, gh_ref.repo, branch
    );
    let resp = client
        .get(&tree_url)
        .send()
        .map_err(|e| format!("Network error fetching repo tree: {}", e))?;

    if resp.status() == 403 {
        return Err("GitHub rate limit reached (60/hr unauthenticated) — try again later.".to_string());
    }
    if resp.status() == 404 {
        return Err(format!("Branch '{}' not found in {}/{}.", branch, gh_ref.owner, gh_ref.repo));
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }

    let tree_resp: GitHubTreeResponse = resp.json()
        .map_err(|e| format!("Failed to parse tree response: {}", e))?;

    if tree_resp.truncated {
        return Err("Repo too large to list — point directly at the skill's subdirectory (not supported in v1's URL parser, so effectively: unsupported).".to_string());
    }

    // Find all SKILL.md paths
    let skill_md_paths: Vec<&str> = tree_resp
        .tree
        .iter()
        .filter(|item| item.item_type == "blob" && item.path.ends_with("/SKILL.md") || item.path == "SKILL.md")
        .map(|item| item.path.as_str())
        .collect();

    if skill_md_paths.is_empty() {
        return Err(format!("No SKILL.md found in {}/{}.", gh_ref.owner, gh_ref.repo));
    }

    // Build candidates: for each SKILL.md, collect all sibling files
    let all_blob_paths: Vec<&str> = tree_resp
        .tree
        .iter()
        .filter(|item| item.item_type == "blob")
        .map(|item| item.path.as_str())
        .collect();

    let mut candidates = Vec::new();
    for skill_md_path in skill_md_paths {
        let dir_path = if let Some(parent) = skill_md_path.strip_suffix("/SKILL.md") {
            parent.to_string()
        } else {
            // SKILL.md is at the root
            String::new()
        };

        let files: Vec<String> = all_blob_paths
            .iter()
            .filter(|&&p| {
                if dir_path.is_empty() {
                    !p.contains('/')
                } else {
                    p.starts_with(&format!("{}/", dir_path))
                }
            })
            .map(|&p| p.to_string())
            .collect();

        let name = if dir_path.is_empty() {
            gh_ref.repo.clone()
        } else {
            dir_path
                .split('/')
                .last()
                .unwrap_or(&dir_path)
                .to_string()
        };

        candidates.push(SkillCandidate {
            dir_path,
            name,
            skill_md_path: skill_md_path.to_string(),
            files,
        });
    }

    Ok(SearchResult {
        owner: gh_ref.owner,
        repo: gh_ref.repo,
        branch,
        candidates,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallGithubRequest {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub candidate: SkillCandidate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallGithubResult {
    pub candidate_name: String,
    pub skill_name: String,       // final installed name (may differ due to auto-rename)
    pub original_name: String,    // desired name before rename
    pub installed_path: String,
    pub files_written: i64,
    pub success: bool,
    pub error: Option<String>,
}

/// Install a single GitHub skill candidate by downloading its files.
pub fn install_github_candidate(req: &InstallGithubRequest, agent: &str) -> Result<InstallGithubResult, String> {
    let client = build_client()?;

    // First, fetch and validate SKILL.md
    let skill_md_raw_url = build_raw_url(
        &req.owner,
        &req.repo,
        &req.branch,
        &req.candidate.skill_md_path,
    );
    let skill_md_content = fetch_raw_file(&client, &skill_md_raw_url)?;
    validate_skill_md(&skill_md_content)
        .map_err(|e| format!("SKILL.md validation failed: {}", e))?;

    let skills_dir = resolve_skills_dir(agent)?;
    let original_name = req.candidate.name.clone();
    let final_name = resolve_unique_name(&skills_dir, &original_name);

    // Download all files into a temp directory
    let temp_dir = skills_dir.join(format!(".tmp-github-{}", final_name));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Cannot clean temp dir: {}", e))?;
    }
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Cannot create temp dir: {}", e))?;

    let mut files_written = 0i64;
    let dir_prefix = if req.candidate.dir_path.is_empty() {
        String::new()
    } else {
        format!("{}/", req.candidate.dir_path)
    };

    for file_path in &req.candidate.files {
        let raw_url = build_raw_url(&req.owner, &req.repo, &req.branch, file_path);
        let content = fetch_raw_bytes(&client, &raw_url)?;

        // Compute relative path within the skill directory
        let relative = if dir_prefix.is_empty() {
            file_path.as_str()
        } else {
            file_path.strip_prefix(&dir_prefix).unwrap_or(file_path)
        };

        let dest = temp_dir.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create dir: {}", e))?;
        }
        std::fs::write(&dest, &content)
            .map_err(|e| format!("Cannot write file {}: {}", dest.display(), e))?;
        files_written += 1;
    }

    // Atomic rename
    let dest_dir = skills_dir.join(&final_name);
    std::fs::rename(&temp_dir, &dest_dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&temp_dir);
        format!("Cannot move skill to destination: {}", e)
    })?;

    Ok(InstallGithubResult {
        candidate_name: req.candidate.name.clone(),
        skill_name: final_name.clone(),
        original_name,
        installed_path: dest_dir.to_string_lossy().to_string(),
        files_written,
        success: true,
        error: None,
    })
}

fn build_raw_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        owner, repo, branch, path
    )
}

fn fetch_raw_file(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    let bytes = fetch_raw_bytes(client, url)?;
    String::from_utf8(bytes).map_err(|e| format!("File is not valid UTF-8: {}", e))
}

fn fetch_raw_bytes(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Failed to fetch {}: HTTP {}", url, resp.status()));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read response bytes: {}", e))
}

fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("codex-skill-installer/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}
