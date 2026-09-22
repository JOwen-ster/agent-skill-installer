<script lang="ts">
  import type { DropInstallResult, InstalledSkill } from '$lib/types';
  import { getInstalledSkills, getSkillsDir } from '$lib/api';
  import { session } from '$lib/stores.svelte';

  import DropZone from '$lib/components/DropZone.svelte';
  import SkillsSearch from '$lib/components/SkillsSearch.svelte';
  import CollapsibleSection from '$lib/components/CollapsibleSection.svelte';
  import GithubInputStack from '$lib/components/GithubInputStack.svelte';
  import UploadsList from '$lib/components/UploadsList.svelte';
  import InstalledSkillsList from '$lib/components/InstalledSkillsList.svelte';
  import HistoryPanel from '$lib/components/HistoryPanel.svelte';

  // ── Top-level state ──────────────────────────────────────────────────────────
  let historyPanel: ReturnType<typeof HistoryPanel> | null = null;

  // ── Lifecycle & Reactivity ───────────────────────────────────────────────────
  let requestId = 0;

  $effect(() => {
      const agent = session.activeAgent;
      refreshData(agent);
  });

  async function refreshData(agent: string = session.activeAgent) {
      const id = ++requestId;
      try {
        const skills = await getInstalledSkills(agent);
        if (id === requestId) session.setInstalledSkills(skills);
      } catch (e) {
        console.error('Could not list installed skills:', e);
      }

      try {
        const dir = await getSkillsDir(agent);
        if (id === requestId) session.setSkillsDir(dir);
      } catch (e) {
        console.error('Could not get skills dir:', e);
      }
  }

  // ── Callbacks ────────────────────────────────────────────────────────────────
  function handleDropResults(results: DropInstallResult[]) {
    session.addUploadResults(results);
    refreshData();
    historyPanel?.refresh();
  }

  function handleGithubInstalled() {
    refreshData();
    historyPanel?.refresh();
  }
</script>

<div class="app-shell" class:claude-mode={session.activeAgent === 'claude'}>
  <header class="app-header">
    <div class="header-content">
      <div>
        <h1 class="app-title">
          <span class="icon">🧩</span> {session.activeAgent === 'claude' ? 'Claude' : 'Codex'} Skill Installer
        </h1>
        <p class="app-subtitle">Install skills into <code>~/.{session.activeAgent}/skills/</code> from GitHub or by drag &amp; drop</p>
      </div>
      
      <div class="agent-toggle">
        <button 
          class="toggle-btn codex-btn" 
          class:active={session.activeAgent === 'codex'}
          onclick={() => session.setActiveAgent('codex')}>
          Codex
        </button>
        <button 
          class="toggle-btn claude-btn" 
          class:active={session.activeAgent === 'claude'}
          onclick={() => session.setActiveAgent('claude')}>
          Claude
        </button>
      </div>
    </div>
  </header>

  <main class="app-main">
    <!-- Drop zone -->
    <section class="section">
      <DropZone onresults={handleDropResults} />
    </section>

    <!-- skills.sh search and bulk queue -->
    <section class="section">
      <SkillsSearch oninstalled={handleGithubInstalled} />
    </section>

    <!-- GitHub URL inputs -->
    <section class="section">
      <CollapsibleSection
        title="Install from GitHub Repository"
        description="Paste a GitHub repository URL to find and install skills."
      >
        <GithubInputStack oninstalled={handleGithubInstalled} />
      </CollapsibleSection>
    </section>

    <!-- Session upload results -->
    {#if session.uploadResults.length > 0}
      <section class="section">
        <UploadsList results={session.uploadResults} />
      </section>
    {/if}

    <!-- Installed skills (live from filesystem) -->
    <section class="section">
      <InstalledSkillsList 
        skills={session.installedSkills} 
        skillsDir={session.skillsDir} 
        onrefresh={() => refreshData()}
      />
    </section>

    <!-- History (collapsible, from SQLite) -->
    <section class="section">
      <HistoryPanel bind:this={historyPanel} />
    </section>
  </main>
</div>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(:root) {
    --accent: #4a9eff;
    --border-color: #e0e0e0;
    --text-primary: #1a1a1a;
    --text-secondary: #777;
    --input-bg: #fff;
    --btn-bg: #ebebeb;
    --btn-hover-bg: #d8d8d8;
    --hover-bg: rgba(0, 0, 0, 0.025);
    --code-bg: rgba(0, 0, 0, 0.07);
    --success: #22c55e;
    --success-bg: rgba(34, 197, 94, 0.08);
    --error: #ef4444;
    --error-bg: rgba(239, 68, 68, 0.08);
    --dropzone-bg: #fafafa;
    --dropzone-active-bg: rgba(74, 158, 255, 0.06);

    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    font-size: 15px;
    line-height: 1.5;
    color: var(--text-primary);
    background: #f5f5f5;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --accent: #5aaeff;
      --border-color: #383838;
      --text-primary: #e8e8e8;
      --text-secondary: #999;
      --input-bg: #2a2a2a;
      --btn-bg: #333;
      --btn-hover-bg: #444;
      --hover-bg: rgba(255, 255, 255, 0.04);
      --code-bg: rgba(255, 255, 255, 0.09);
      --success: #4ade80;
      --success-bg: rgba(74, 222, 128, 0.1);
      --error: #f87171;
      --error-bg: rgba(248, 113, 113, 0.1);
      --dropzone-bg: #1e1e1e;
      --dropzone-active-bg: rgba(90, 174, 255, 0.1);

      background: #181818;
      color: var(--text-primary);
    }
  }

  :global(body) {
    margin: 0;
    padding: 0;
    min-height: 100vh;
    background: inherit;
  }

  :global(input), :global(button) {
    font-family: inherit;
  }

  .app-shell {
    max-width: 800px;
    margin: 0 auto;
    padding: 1.25rem 1.5rem 2rem;
    min-height: 100vh;
  }

  .app-header {
    margin-bottom: 1.5rem;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 1rem;
  }

  .app-title {
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0 0 0.25rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-primary);
  }

  .icon {
    font-size: 1.2rem;
  }

  .app-subtitle {
    font-size: 0.84rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .app-subtitle code {
    font-size: 0.9em;
    background: var(--code-bg);
    padding: 0.1em 0.35em;
    border-radius: 3px;
  }

  .app-main {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .agent-toggle {
    display: flex;
    background: var(--btn-bg);
    border-radius: 6px;
    padding: 3px;
    gap: 2px;
  }

  .toggle-btn {
    appearance: none;
    background: transparent;
    border: none;
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .toggle-btn:hover {
    color: var(--text-primary);
  }

  .toggle-btn.codex-btn.active {
    background: #4a9eff;
    color: #fff;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .toggle-btn.claude-btn.active {
    background: #f97316; /* Claude orange */
    color: #fff;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  /* When Claude mode is active, change the global accent color */
  .app-shell.claude-mode {
    --accent: #f97316;
    --dropzone-active-bg: rgba(249, 115, 22, 0.06);
  }

  @media (prefers-color-scheme: dark) {
    .app-shell.claude-mode {
      --accent: #fb923c;
      --dropzone-active-bg: rgba(251, 146, 60, 0.1);
    }
  }
</style>
