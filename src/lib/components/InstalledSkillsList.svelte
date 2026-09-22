<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { session } from "$lib/stores.svelte";
  import { deleteSkill } from "$lib/api";
  import ConfirmModal from "./ConfirmModal.svelte";
  import CollapsibleSection from './CollapsibleSection.svelte';

  interface InstalledSkill {
    name: string;
    path: string;
    description?: string | null;
    source_repo?: string | null;
  }

  let {
    skills = [],
    skillsDir = null,
    onrefresh = () => {}
  }: {
    skills: InstalledSkill[];
    skillsDir: string | null;
    onrefresh: () => void;
  } = $props();

  let searchQuery = $state("");

  let filteredSkills = $derived(
    skills.filter(s => {
      if (!searchQuery) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.name.toLowerCase().includes(q) ||
        (s.description && s.description.toLowerCase().includes(q)) ||
        (s.source_repo && s.source_repo.toLowerCase().includes(q))
      );
    })
  );

  let { groups, sortedRepoNames, local } = $derived.by(() => {
    const groups: Record<string, InstalledSkill[]> = {};
    const local: InstalledSkill[] = [];

    for (const skill of filteredSkills) {
      if (skill.source_repo) {
        if (!groups[skill.source_repo]) groups[skill.source_repo] = [];
        groups[skill.source_repo].push(skill);
      } else {
        local.push(skill);
      }
    }

    const sortedRepoNames = Object.keys(groups).sort();
    return { groups, sortedRepoNames, local };
  });

  function displayPath(path: string) {
    if (!skillsDir) return path;
    if (path === skillsDir) return path;
    if (path.startsWith(skillsDir)) {
      let rel = path.substring(skillsDir.length);
      if (rel.startsWith("/") || rel.startsWith("\\")) {
        rel = rel.substring(1);
      }
      return rel;
    }
    return path;
  }

  async function openFolder() {
    await invoke("open_skills_dir", { agent: session.activeAgent }).catch(e => console.error("Failed to open skills dir:", e));
  }

  let modal: ConfirmModal;
  let skillToDelete: InstalledSkill | null = $state(null);
  let isBulkDelete = $state(false);
  let deleteAgent: "codex" | "claude" | null = $state(null);

  function promptDelete(skill: InstalledSkill) {
    isBulkDelete = false;
    deleteAgent = session.activeAgent;
    skillToDelete = skill;
    modal.show();
  }

  function promptBulkDelete() {
    isBulkDelete = true;
    deleteAgent = session.activeAgent;
    modal.show();
  }

  async function confirmDelete() {
    if (isBulkDelete) {
      const agent = deleteAgent;
      if (!agent) return;
      const failedSkills = new Set<string>();
      for (const skillName of selectedSkills) {
        try {
          await deleteSkill(agent, skillName);
        } catch (e) {
          console.error(`Failed to delete skill ${skillName}:`, e);
          failedSkills.add(skillName);
        }
      }
      selectedSkills = failedSkills;
      onrefresh();
    } else {
      if (!skillToDelete) return;
      const agent = deleteAgent;
      if (!agent) return;
      try {
        await deleteSkill(agent, skillToDelete.name);
        selectedSkills.delete(skillToDelete.name);
        selectedSkills = new Set(selectedSkills);
        onrefresh();
      } catch (e) {
        console.error("Failed to delete skill:", e);
      }
      skillToDelete = null;
    }
    isBulkDelete = false;
    deleteAgent = null;
  }

  let selectedSkills = $state(new Set<string>());

  function toggleSkill(name: string, checked: boolean) {
    if (checked) {
      selectedSkills.add(name);
    } else {
      selectedSkills.delete(name);
    }
    selectedSkills = new Set(selectedSkills);
  }

  function toggleRepo(repoName: string, checked: boolean) {
    const repoSkills = groups[repoName] || [];
    for (const s of repoSkills) {
      if (checked) {
        selectedSkills.add(s.name);
      } else {
        selectedSkills.delete(s.name);
      }
    }
    selectedSkills = new Set(selectedSkills);
  }

  function isRepoFullySelected(repoName: string) {
    const repoSkills = groups[repoName] || [];
    if (repoSkills.length === 0) return false;
    return repoSkills.every(s => selectedSkills.has(s.name));
  }

  function isRepoPartiallySelected(repoName: string) {
    const repoSkills = groups[repoName] || [];
    if (repoSkills.length === 0) return false;
    const selectedCount = repoSkills.filter(s => selectedSkills.has(s.name)).length;
    return selectedCount > 0 && selectedCount < repoSkills.length;
  }
</script>

{#snippet skillItem(skill: InstalledSkill)}
  <li class="skill-item">
    <div class="skill-info">
      <input 
        type="checkbox" 
        class="skill-checkbox" 
        checked={selectedSkills.has(skill.name)} 
        onchange={(e) => toggleSkill(skill.name, e.currentTarget.checked)}
      />
      <span class="skill-name">{skill.name}</span>
      {#if skill.description}
        <span class="skill-desc">{skill.description}</span>
      {/if}
      <span class="skill-path" title={skill.path}>{displayPath(skill.path)}</span>
    </div>
    <button class="delete-btn" title="Delete skill" onclick={() => promptDelete(skill)}>
      <svg style="width:1.6em; height:1.6em; display:block;" viewBox="-2.5 0 61 61" xmlns="http://www.w3.org/2000/svg" fill="currentColor"><path fill-rule="evenodd" d="M36 26v10.997c0 1.659-1.337 3.003-3.009 3.003h-9.981c-1.662 0-3.009-1.342-3.009-3.003v-10.997h16zm-2 0v10.998c0 .554-.456 1.002-1.002 1.002h-9.995c-.554 0-1.002-.456-1.002-1.002v-10.998h12zm-9-5c0-.552.451-1 .991-1h4.018c.547 0 .991.444.991 1 0 .552-.451 1-.991 1h-4.018c-.547 0-.991-.444-.991-1zm0 6.997c0-.551.444-.997 1-.997.552 0 1 .453 1 .997v6.006c0 .551-.444.997-1 .997-.552 0-1-.453-1-.997v-6.006zm4 0c0-.551.444-.997 1-.997.552 0 1 .453 1 .997v6.006c0 .551-.444.997-1 .997-.552 0-1-.453-1-.997v-6.006zm-6-5.997h-4.008c-.536 0-.992.448-.992 1 0 .556.444 1 .992 1h18.016c.536 0 .992-.448.992-1 0-.556-.444-1-.992-1h-4.008v-1c0-1.653-1.343-3-3-3h-3.999c-1.652 0-3 1.343-3 3v1z"></path></svg>
    </button>
  </li>
{/snippet}

<CollapsibleSection title="Currently Installed Skills">
  {#snippet actions()}
    <div class="header-actions">
      {#if selectedSkills.size > 0}
        <button
          class="bulk-delete-btn"
          onclick={promptBulkDelete}
          title="Delete selected skills"
        >
          <svg style="width:1.5em; height:1.5em; margin-right: 0.2rem;" viewBox="-2.5 0 61 61" xmlns="http://www.w3.org/2000/svg" fill="currentColor"><path fill-rule="evenodd" d="M36 26v10.997c0 1.659-1.337 3.003-3.009 3.003h-9.981c-1.662 0-3.009-1.342-3.009-3.003v-10.997h16zm-2 0v10.998c0 .554-.456 1.002-1.002 1.002h-9.995c-.554 0-1.002-.456-1.002-1.002v-10.998h12zm-9-5c0-.552.451-1 .991-1h4.018c.547 0 .991.444.991 1 0 .552-.451 1-.991 1h-4.018c-.547 0-.991-.444-.991-1zm0 6.997c0-.551.444-.997 1-.997.552 0 1 .453 1 .997v6.006c0 .551-.444.997-1 .997-.552 0-1-.453-1-.997v-6.006zm4 0c0-.551.444-.997 1-.997.552 0 1 .453 1 .997v6.006c0 .551-.444.997-1 .997-.552 0-1-.453-1-.997v-6.006zm-6-5.997h-4.008c-.536 0-.992.448-.992 1 0 .556.444 1 .992 1h18.016c.536 0 .992-.448.992-1 0-.556-.444-1-.992-1h-4.008v-1c0-1.653-1.343-3-3-3h-3.999c-1.652 0-3 1.343-3 3v1z"></path></svg>
          Delete ({selectedSkills.size})
        </button>
      {/if}
      <input 
        type="text" 
        class="search-input" 
        placeholder="Search skills..." 
        bind:value={searchQuery} 
      />
      <button 
        class="open-btn" 
        onclick={openFolder} 
        title="Open in File Explorer"
      >
        📁
      </button>
    </div>
  {/snippet}

  {#if skills.length === 0}
    <p class="empty-note">No skills installed yet.</p>
  {:else if filteredSkills.length === 0}
    <p class="empty-note">No skills match your search.</p>
  {:else}
    <div class="skills-groups">
      {#if local.length > 0}
        <ul class="skills-list">
          {#each local as skill}
            {@render skillItem(skill)}
          {/each}
        </ul>
      {/if}

      {#each sortedRepoNames as repoName}
        <div class="repo-group">
          <h4 class="repo-header">
            <input 
              type="checkbox" 
              class="repo-checkbox"
              checked={isRepoFullySelected(repoName)}
              indeterminate={isRepoPartiallySelected(repoName)}
              onchange={(e) => toggleRepo(repoName, e.currentTarget.checked)}
            />
            <span class="repo-icon">github:</span>
            {repoName}
          </h4>
          <ul class="skills-list">
            {#each groups[repoName] as skill}
              {@render skillItem(skill)}
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {/if}

  {#if skillsDir}
    <p class="skills-dir-hint">
      <span class="dir-label">Skills directory:</span>
      <code>{displayPath(skillsDir)}</code>
    </p>
  {/if}

  <ConfirmModal 
    bind:this={modal}
    title={isBulkDelete ? "Delete Selected Skills" : "Delete Skill"}
    message={isBulkDelete
      ? `Are you sure you want to delete ${selectedSkills.size} selected skill(s)? This action cannot be undone.`
      : `Are you sure you want to delete "${skillToDelete?.name}"? This action cannot be undone.`}
    confirmText="Delete"
    onconfirm={confirmDelete}
    oncancel={() => { skillToDelete = null; isBulkDelete = false; deleteAgent = null; }}
  />
</CollapsibleSection>

<style>
  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .bulk-delete-btn {
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
    font-weight: 500;
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 4px;
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    transition: background 0.15s, border-color 0.15s;
  }

  .bulk-delete-btn:hover {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.6);
  }

  .search-input {
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 4px;
    background: var(--input-bg, #fff);
    color: var(--text-primary, #222);
    min-width: 150px;
  }

  .open-btn {
    padding: 0.35rem 0.5rem;
    font-size: 0.9rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 4px;
    background: var(--btn-bg, #ebebeb);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
  }

  .open-btn:hover {
    background: var(--btn-hover-bg, #d8d8d8);
  }

  .empty-note {
    font-size: 0.85rem;
    color: var(--text-secondary, #aaa);
    margin: 0;
    padding: 0.25rem 0.5rem;
    font-style: italic;
  }

  .skills-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .skill-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.5rem;
    border-radius: 5px;
    font-size: 0.88rem;
    border: 1px solid transparent;
    transition: border-color 0.1s;
  }

  .skill-info {
    display: grid;
    grid-template-columns: auto minmax(120px, auto) 1fr auto;
    gap: 0.5rem;
    align-items: center;
    flex-grow: 1;
    min-width: 0;
  }
  
  .skill-checkbox {
    margin: 0;
    cursor: pointer;
    width: 14px;
    height: 14px;
  }

  .delete-btn {
    background: transparent;
    border: none;
    font-size: 1.1rem;
    line-height: 1;
    color: var(--error, #ef4444);
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    opacity: 0.6;
    transition: opacity 0.2s, background 0.2s;
  }

  .delete-btn:hover {
    opacity: 1;
    color: var(--error, #ef4444);
    background: var(--error-bg, rgba(239, 68, 68, 0.1));
  }

  .skill-item:hover {
    border-color: var(--border-color, #ddd);
    background: var(--hover-bg, rgba(0,0,0,0.03));
  }

  .skill-name {
    font-weight: 600;
    color: var(--text-primary, #222);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .skill-desc {
    font-size: 0.82rem;
    color: var(--text-secondary, #777);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skill-path {
    font-size: 0.75rem;
    color: var(--text-secondary, #aaa);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: "Cascadia Code", "JetBrains Mono", monospace;
    justify-self: end;
  }

  .skills-dir-hint {
    margin: 0.6rem 0 0;
    font-size: 0.78rem;
    color: var(--text-secondary, #aaa);
  }

  .dir-label {
    margin-right: 0.25rem;
  }

  code {
    font-size: 0.9em;
    background: var(--code-bg, rgba(0,0,0,0.06));
    padding: 0.1em 0.35em;
    border-radius: 3px;
  }

  .skills-groups {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .repo-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .repo-header {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-primary, #222);
    margin: 0 0 0.2rem 0;
    padding-left: 0.2rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .repo-checkbox {
    margin: 0;
    cursor: pointer;
    width: 14px;
    height: 14px;
  }

  .repo-icon {
    color: var(--text-secondary, #888);
    font-weight: normal;
  }
</style>
