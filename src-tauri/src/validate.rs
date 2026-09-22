/// SKILL.md validation logic.
///
/// A valid SKILL.md must:
/// 1. Have a YAML frontmatter block delimited by `---` lines.
/// 2. The frontmatter must contain at least `name` or `description`.
/// 3. The body after the frontmatter must be non-empty.

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("No YAML frontmatter found (file must start with ---)")]
    NoFrontmatter,
    #[error("Frontmatter is missing both 'name' and 'description' fields")]
    MissingRequiredFields,
    #[error("Body content after frontmatter is empty")]
    EmptyBody,
    #[error("Failed to parse YAML frontmatter: {0}")]
    YamlParseFailed(String),
}
/// Validates the content of a SKILL.md file.
/// Returns `Ok(skill_name)` if valid, or a `ValidationError` describing the problem.
pub fn validate_skill_md(content: &str) -> Result<String, ValidationError> {
    // Must start with ---
    if !content.trim_start().starts_with("---") {
        return Err(ValidationError::NoFrontmatter);
    }

    let content = content.trim_start();
    // Find the closing ---
    let after_open = &content[3..]; // skip the first ---
    // skip optional newline
    let after_open = after_open.trim_start_matches('\r').trim_start_matches('\n');

    let close_pos = after_open.find("\n---")
        .or_else(|| after_open.find("\r\n---"));

    let (frontmatter_str, body) = match close_pos {
        None => return Err(ValidationError::NoFrontmatter),
        Some(pos) => {
            let fm = &after_open[..pos];
            // Skip past the closing ---
            let rest = &after_open[pos..];
            let body_start = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
            let body = &rest[body_start..];
            (fm, body)
        }
    };

    // Parse the frontmatter YAML
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| ValidationError::YamlParseFailed(e.to_string()))?;

    // Check for name or description
    let name_opt = yaml.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let has_description = yaml.get("description").map(|v| !v.is_null()).unwrap_or(false);

    if name_opt.is_none() && !has_description {
        return Err(ValidationError::MissingRequiredFields);
    }

    // Body must be non-empty
    if body.trim().is_empty() {
        return Err(ValidationError::EmptyBody);
    }

    Ok(name_opt.unwrap_or_else(|| "unnamed-skill".to_string()))
}
