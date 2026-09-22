// Typed wrappers around invoke() — single point of contact between frontend and Rust.

import { invoke } from '@tauri-apps/api/core';
import type {
  DropInstallResult,
  HistoryData,
  InstallGithubRequest,
  InstallGithubResult,
  InstalledSkill,
  SearchResult,
  SkillsSearchItem,
} from './types';

export async function searchGithub(url: string, agent: string): Promise<SearchResult> {
  return invoke<SearchResult>('search_github', { url, agent });
}

export async function searchSkills(query: string): Promise<SkillsSearchItem[]> {
  return invoke<SkillsSearchItem[]>('search_skills', { query });
}

export async function installGithubSkills(
  requests: InstallGithubRequest[],
  agent: string
): Promise<InstallGithubResult[]> {
  return invoke<InstallGithubResult[]>('install_github_skills', { requests, agent });
}

export async function installDroppedPaths(
  paths: string[],
  agent: string
): Promise<DropInstallResult[]> {
  return invoke<DropInstallResult[]>('install_dropped_paths', { paths, agent });
}

export async function getInstalledSkills(agent: string): Promise<InstalledSkill[]> {
  return invoke<InstalledSkill[]>('get_installed_skills', { agent });
}

export async function getHistory(): Promise<HistoryData> {
  return invoke<HistoryData>('get_history'); // Does not need agent right now according to backend
}

export async function getSkillsDir(agent: string): Promise<string> {
  return invoke<string>('get_skills_dir', { agent });
}

export async function deleteSkill(agent: string, skillName: string): Promise<void> {
  return invoke<void>('delete_skill', { agent, skillName });
}
