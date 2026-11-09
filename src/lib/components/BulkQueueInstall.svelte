<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    InstallGithubRequest,
    InstallGithubResult,
    SearchResult,
    SkillCandidate,
    SkillsSearchItem,
  } from '$lib/types';
  import { installGithubSkills, searchGithub } from '$lib/api';
  import { findInstalledGithubSkill } from '$lib/installed';
  import { session } from '$lib/stores.svelte';
  import CollapsibleSection from './CollapsibleSection.svelte';
  import SkillSelect from './SkillSelect.svelte';

  interface Props {
    skills: SkillsSearchItem[];
    oninstalled: () => void;
    busy?: boolean;
  }

  let { skills, oninstalled, busy = $bindable(false) }: Props = $props();

  type PanelStatus = 'searching' | 'done' | 'error';

  interface Panel {
    key: string;
    item: SkillsSearchItem;
    preselectedNames: string[];
    status: PanelStatus;
    error: string | null;
    result: SearchResult | null;
    selected: SkillCandidate[];
    installResults: (InstallGithubResult | null)[];
    installing: boolean;
    progress: string;
  }

  let panels = $state<Panel[]>([]);
  let bulkInstalling = $state(false);
  let bulkProgress = $state('');
  let bulkError = $state<string | null>(null);

  $effect(() => {
    busy = bulkInstalling || panels.some((panel) => panel.installing);
  });

  function panelKey(item: SkillsSearchItem): string {
    return `${item.github_url}#${item.skill_slug}`;
  }

  function makePanel(item: SkillsSearchItem): Panel {
    return {
      key: panelKey(item),
      item,
      preselectedNames: [item.skill_name, item.skill_slug],
      status: 'searching',
      error: null,
      result: null,
      selected: [],
      installResults: [],
      installing: false,
      progress: '',
    };
  }

  $effect(() => {
    const nextPanels = skills.map(makePanel);
    panels = nextPanels;
    for (const panel of nextPanels) {
      // Changing the agent must not reset panels during an active install.
      void untrack(() => loadPanel(panel.key, panel.item.github_url));
    }
  });

  let selectableCount = $derived(
    panels.reduce((total, panel) => total + requestsFor(panel).length, 0)
  );

  function updatePanel(key: string, update: Partial<Panel>) {
    panels = panels.map((panel) => (panel.key === key ? { ...panel, ...update } : panel));
  }

  async function loadPanel(key: string, url: string) {
    try {
      const result = await searchGithub(url, session.activeAgent);
      updatePanel(key, {
        status: 'done',
        result,
        installResults: result.candidates.map(() => null),
        selected: [],
        error: null,
      });
    } catch (e) {
      updatePanel(key, {
        status: 'error',
        error: String(e),
        result: null,
        selected: [],
      });
    }
  }

  function handleSelection(key: string, candidates: SkillCandidate[]) {
    updatePanel(key, { selected: candidates });
  }

  function resultFor(panel: Panel, candidate: SkillCandidate): InstallGithubResult | null {
    return panel.installResults.find((item) => item?.candidate_name === candidate.name) ?? null;
  }

  function requestsFor(panel: Panel): InstallGithubRequest[] {
    if (!panel.result) return [];

    return panel.selected
      .filter((candidate) => resultFor(panel, candidate) === null
        && !findInstalledGithubSkill(
          session.installedSkills,
          `https://github.com/${panel.result!.owner}/${panel.result!.repo}`,
          candidate.name
        ))
      .map((candidate) => ({
        owner: panel.result!.owner,
        repo: panel.result!.repo,
        branch: panel.result!.branch,
        candidate,
      }));
  }

  function mergeResults(panel: Panel, results: InstallGithubResult[]): (InstallGithubResult | null)[] {
    const resultMap = new Map(results.map((item) => [item.candidate_name, item]));
    return panel.result?.candidates.map((candidate) => {
      return resultMap.get(candidate.name)
        ?? panel.installResults.find((item) => item?.candidate_name === candidate.name)
        ?? null;
    }) ?? panel.installResults;
  }

  async function installPanel(key: string) {
    if (bulkInstalling) return;
    const panel = panels.find((item) => item.key === key);
    if (!panel || panel.installing) return;

    const requests = requestsFor(panel);
    if (requests.length === 0) return;

    updatePanel(key, { installing: true, progress: `0 of ${requests.length} complete`, error: null });
    try {
      await installGithubSkills(requests, session.activeAgent, (result, index) => {
        const current = panels.find((item) => item.key === key);
        if (current) updatePanel(key, {
          installResults: mergeResults(current, [result]),
          progress: `${index + 1} of ${requests.length} complete`,
        });
      });
    } catch (e) {
      updatePanel(key, { error: String(e) });
    } finally {
      updatePanel(key, { installing: false });
      oninstalled();
    }
  }

  async function installAll() {
    if (bulkInstalling || panels.some((panel) => panel.installing)) return;

    const jobs = panels.flatMap((panel) =>
      requestsFor(panel).map((request) => ({ panelKey: panel.key, request }))
    );
    if (jobs.length === 0) return;

    bulkInstalling = true;
    bulkProgress = `0 of ${jobs.length} complete`;
    bulkError = null;
    panels = panels.map((panel) => ({ ...panel, installing: true, error: null }));

    try {
      await installGithubSkills(
        jobs.map((job) => job.request),
        session.activeAgent,
        (result, index) => {
          bulkProgress = `${index + 1} of ${jobs.length} complete`;
          const key = jobs[index].panelKey;
          const panel = panels.find((item) => item.key === key);
          if (panel) updatePanel(key, { installResults: mergeResults(panel, [result]) });
        }
      );
    } catch (e) {
      bulkError = String(e);
    } finally {
      panels = panels.map((panel) => ({ ...panel, installing: false }));
      bulkInstalling = false;
      oninstalled();
    }
  }

  function clearPanel(key: string) {
    if (bulkInstalling || panels.find((panel) => panel.key === key)?.installing) return;
    panels = panels.filter((panel) => panel.key !== key);
  }

  function clearAllPanels() {
    if (bulkInstalling || panels.some((panel) => panel.installing)) return;
    panels = [];
  }
</script>

{#if panels.length > 0}
  <div class="bulk-install">
    <CollapsibleSection
      title="Queued skill installs"
      description="Review the GitHub candidates before installing."
    >
      {#snippet actions()}
        <button class="clear-btn" type="button" onclick={clearAllPanels} disabled={bulkInstalling || panels.some((panel) => panel.installing)}>
          Clear all panels
        </button>
        <button class="bulk-btn" onclick={installAll} disabled={bulkInstalling || panels.some((panel) => panel.installing) || selectableCount === 0}>
          {#if bulkInstalling}
            <span class="spinner" aria-hidden="true"></span> Installing
          {:else}
            Install all selected ({selectableCount})
          {/if}
        </button>
      {/snippet}

    {#if bulkInstalling}
      <p role="status"><span class="spinner" aria-hidden="true"></span> Installing {bulkProgress}</p>
    {/if}
    {#if bulkError}
      <p class="panel-message error" role="alert">{bulkError}</p>
    {/if}

    <div class="panels">
      {#each panels as panel (panel.key)}
        <article class="panel">
          <div class="panel-heading">
            <div>
              <strong>{panel.item.skill_name}</strong>
              <span class="install-count">{panel.item.installs.toLocaleString()} installs</span>
            </div>
            <div class="panel-heading-actions">
              <a class="github-url" href={panel.item.github_url} target="_blank" rel="noreferrer">
                {panel.item.github_url}
              </a>
              <button
                class="clear-btn"
                type="button"
                onclick={() => clearPanel(panel.key)}
                disabled={panel.installing || bulkInstalling}
                aria-label={`Clear ${panel.item.skill_name} install panel`}
              >
                Clear
              </button>
            </div>
          </div>

          {#if panel.status === 'searching'}
            <p class="panel-message">Searching GitHub…</p>
          {:else if panel.status === 'error'}
            <p class="panel-message error">⚠ {panel.error}</p>
          {:else if panel.result}
            {#if panel.error}
              <p class="panel-message error" role="alert">{panel.error}</p>
            {/if}
            {#if panel.installing && !bulkInstalling}
              <p role="status"><span class="spinner" aria-hidden="true"></span> Installing {panel.progress}</p>
            {/if}
            <SkillSelect
              result={panel.result}
              installedSkills={session.installedSkills}
              preselectedNames={panel.preselectedNames}
              installResults={panel.installResults}
              installing={panel.installing || bulkInstalling}
              onselectionchange={(candidates) => handleSelection(panel.key, candidates)}
            />

            <button
              class="panel-btn"
              onclick={() => installPanel(panel.key)}
              disabled={panel.installing || bulkInstalling || requestsFor(panel).length === 0}
            >
              {#if panel.installing}
                <span class="spinner" aria-hidden="true"></span> Installing
              {:else}
                Install selected ({requestsFor(panel).length})
              {/if}
            </button>
          {/if}
        </article>
      {/each}
    </div>
    </CollapsibleSection>
  </div>
{/if}

<style>
  .spinner {
    display: inline-block;
    width: 0.8em;
    height: 0.8em;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    vertical-align: -0.1em;
    margin-inline-end: 0.3em;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; }
  }

  .bulk-install {
    margin-top: 1rem;
  }

  .panel-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
  }

  .panel-heading-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .panel-message {
    margin: 0.15rem 0 0;
    color: var(--text-secondary, #777);
    font-size: 0.82rem;
  }

  .panels {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .panel {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    padding: 0.85rem;
  }

  .panel-heading {
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .panel-heading-actions {
    flex: 1;
    min-width: min(420px, 100%);
  }

  .panel-heading strong {
    display: block;
    font-size: 0.92rem;
  }

  .install-count {
    color: var(--text-secondary, #777);
    font-size: 0.78rem;
  }

  .github-url {
    min-width: min(360px, 100%);
    flex: 1;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 5px;
    background: var(--input-bg, #fff);
    color: var(--text-secondary, #666);
    font-family: 'Cascadia Code', 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.78rem;
  }

  .panel-message.error {
    color: var(--error, #ef4444);
  }

  .clear-btn {
    border: none;
    background: transparent;
    color: var(--text-secondary, #777);
    cursor: pointer;
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .clear-btn:hover {
    color: var(--error, #ef4444);
  }

  .bulk-btn,
  .panel-btn {
    padding: 0.5rem 0.8rem;
    border: none;
    border-radius: 6px;
    background: var(--accent, #4a9eff);
    color: white;
    cursor: pointer;
    font-size: 0.82rem;
  }

  .panel-btn {
    margin-top: 0.6rem;
  }

  .bulk-btn:disabled,
  .panel-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 640px) {
    .panel-heading-actions {
      align-items: stretch;
      flex-direction: column;
    }

    .bulk-btn {
      width: 100%;
    }
  }
</style>
