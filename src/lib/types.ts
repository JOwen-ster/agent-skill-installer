// TypeScript types mirroring Rust structs returned by Tauri commands.

export interface SkillCandidate {
  dir_path: string;
  name: string;
  skill_md_path: string;
  files: string[];
}

export interface SearchResult {
  owner: string;
  repo: string;
  branch: string;
  candidates: SkillCandidate[];
}

export interface SkillsSearchItem {
  skill_name: string;
  skill_slug: string;
  github_url: string;
  installs: number;
}

export interface InstallGithubRequest {
  owner: string;
  repo: string;
  branch: string;
  candidate: SkillCandidate;
}

export interface InstallGithubResult {
  candidate_name: string;
  skill_name: string;
  original_name: string;
  installed_path: string;
  files_written: number;
  success: boolean;
  error: string | null;
}

export interface DropInstallResult {
  path: string;
  skill_name: string;
  original_name: string;
  installed_path: string;
  files_written: number;
  success: boolean;
  error: string | null;
}

export interface InstalledSkill {
  name: string;
  path: string;
  description: string | null;
  source_repo: string | null;
}

export interface SearchRecord {
  id: number;
  github_url: string;
  owner: string;
  repo: string;
  branch: string;
  searched_at: string;
  agent: string;
}

export interface InstallRecord {
  id: number;
  skill_name: string;
  source_type: string;
  source_ref: string | null;
  installed_path: string;
  files_written: number;
  installed_at: string;
  status: string;
  error_message: string | null;
  agent: string;
}

export interface HistoryData {
  searches: SearchRecord[];
  installs: InstallRecord[];
}

// UI-layer types for per-row state in the GitHub input stack
export type SearchStatus = 'idle' | 'searching' | 'done' | 'error';

export interface GithubRowState {
  id: number;
  url: string;
  status: SearchStatus;
  error: string | null;
  result: SearchResult | null;
}
