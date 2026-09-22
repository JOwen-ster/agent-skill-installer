// Shared reactive state using Svelte 5 runes.
// Since Svelte 5 $state() lives in the component scope, we export
// plain objects with reactive fields that can be passed down as props.
// Cross-component state is managed in +page.svelte and passed down.

import type { DropInstallResult, InstalledSkill } from './types';

// Session uploads (drag-and-drop results) — in-memory only, not persisted.
// Exported as a module-level reactive object for sharing across components.
class SessionState {
  uploadResults = $state<DropInstallResult[]>([]);
  installedSkills = $state<InstalledSkill[]>([]);
  skillsDir = $state<string>('');
  activeAgent = $state<'codex' | 'claude'>('codex');

  addUploadResults(results: DropInstallResult[]) {
    this.uploadResults = [...this.uploadResults, ...results];
  }

  setInstalledSkills(skills: InstalledSkill[]) {
    this.installedSkills = skills;
  }

  setSkillsDir(dir: string) {
    this.skillsDir = dir;
  }

  setActiveAgent(agent: 'codex' | 'claude') {
    this.activeAgent = agent;
  }
}

export const session = new SessionState();
