//! skills.sh public search API integration.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::time::{Duration, timeout};

const SKILLS_SEARCH_API_URL: &str = "https://www.skills.sh/api/search";
const RESULT_LIMIT: usize = 10;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SkillsSearchItem {
    pub skill_name: String,
    pub skill_slug: String,
    pub github_url: String,
    pub installs: u64,
}

#[derive(Debug, Deserialize)]
struct SkillsSearchResponse {
    skills: Vec<SkillsSearchApiItem>,
}

#[derive(Debug, Deserialize)]
struct SkillsSearchApiItem {
    #[serde(rename = "skillId")]
    skill_id: String,
    name: String,
    installs: u64,
    source: String,
}

/// Fetch the public, unauthenticated skills.sh endpoint and return the first
/// ten results backed by a valid GitHub `owner/repo` source.
pub async fn search_skills(query: &str) -> Result<Vec<SkillsSearchItem>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Enter a skill search term.".to_string());
    }

    let client = Client::builder()
        .user_agent("agent-skill-installer/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Failed to build skills.sh HTTP client: {error}"))?;

    let response = timeout(
        Duration::from_secs(30),
        client
            .get(SKILLS_SEARCH_API_URL)
            .query(&[("q", query), ("limit", "100")])
            .send(),
    )
    .await
    .map_err(|_| "Timed out fetching skills.sh search results.".to_string())?
    .map_err(|error| format!("Network error fetching skills.sh search: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("skills.sh search returned HTTP {status}."));
    }

    let response = timeout(
        Duration::from_secs(30),
        response.json::<SkillsSearchResponse>(),
    )
    .await
    .map_err(|_| "Timed out reading skills.sh search results.".to_string())?
    .map_err(|error| format!("Failed to parse skills.sh search response: {error}"))?;

    format_search_results(response.skills)
}

fn format_search_results(
    skills: Vec<SkillsSearchApiItem>,
) -> Result<Vec<SkillsSearchItem>, String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for skill in skills {
        let Some(github_url) = github_url_from_source(&skill.source) else {
            continue;
        };

        let skill_name = skill.name.trim();
        let skill_slug = skill.skill_id.trim();
        if skill_name.is_empty() || skill_slug.is_empty() {
            continue;
        }

        let key = format!("{github_url}#{skill_slug}");
        if !seen.insert(key) {
            continue;
        }

        results.push(SkillsSearchItem {
            skill_name: skill_name.to_string(),
            skill_slug: skill_slug.to_string(),
            github_url,
            installs: skill.installs,
        });

        if results.len() == RESULT_LIMIT {
            break;
        }
    }

    if results.is_empty() {
        return Err("No GitHub-backed skills were found for that search.".to_string());
    }

    Ok(results)
}

fn github_url_from_source(source: &str) -> Option<String> {
    let mut parts = source.trim().split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if parts.next().is_some() || !is_github_component(owner) || !is_github_component(repo) {
        return None;
    }

    Some(format!("https://github.com/{owner}/{repo}"))
}

fn is_github_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}
