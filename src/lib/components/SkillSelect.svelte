<script lang="ts">
  import { onMount } from 'svelte';
  import type { InstallGithubResult, SearchResult, SkillCandidate } from '$lib/types';

  interface Props {
    result: SearchResult;
    preselectedNames?: string[];
    installResults?: (InstallGithubResult | null)[];
    installing?: boolean;
    onselectionchange?: (candidates: SkillCandidate[]) => void;
  }

  let {
    result,
    preselectedNames = [],
    installResults = [],
    installing = false,
    onselectionchange = () => {},
  }: Props = $props();

  let checked = $state<boolean[]>([]);
  let initializedSelectionKey = '';
  let selectAllInput: HTMLInputElement | undefined;
  let candidateInputs: (HTMLInputElement | undefined)[] = [];

  function normalize(value: string): string {
    return value.toLowerCase().replace(/[^a-z0-9]+/g, '');
  }

  function candidateKeys(candidate: SkillCandidate): string[] {
    const directoryName = candidate.dir_path.split('/').filter(Boolean).at(-1) ?? '';
    const skillDirectory = candidate.skill_md_path
      .replace(/[/\\]SKILL\.md$/i, '')
      .split('/')
      .filter(Boolean)
      .at(-1) ?? '';

    return [candidate.name, directoryName, skillDirectory].map(normalize).filter(Boolean);
  }

  function matchesPreselection(candidate: SkillCandidate): boolean {
    const candidateNames = candidateKeys(candidate);
    return preselectedNames.some((name) => candidateNames.includes(normalize(name)));
  }

  function initializeChecked(): boolean[] {
    if (preselectedNames.length === 0) {
      return result.candidates.map(() => true);
    }
    return result.candidates.map(matchesPreselection);
  }

  $effect(() => {
    // Parent panel updates can replace the surrounding panel object when a
    // different queue item changes. Only reset the initial preselection when
    // the actual candidate set or queued skill changes; otherwise a parent
    // update would immediately overwrite a user's checkbox click.
    const selectionKey = [
      preselectedNames.join('\u0000'),
      ...result.candidates.map((candidate) =>
        [candidate.name, candidate.dir_path, candidate.skill_md_path].join('\u0000')
      ),
    ].join('\u0001');

    if (selectionKey === initializedSelectionKey) return;

    initializedSelectionKey = selectionKey;
    checked = initializeChecked();
  });

  $effect(() => {
    // Keep the native checkbox properties synchronized after a parent panel
    // update. This is important for queued panels because their keyed DOM
    // nodes can be reused while the selection state is replaced.
    const states = checked;
    result.candidates.forEach((_, index) => {
      const input = candidateInputs[index];
      if (input) input.checked = states[index] ?? false;
    });

    if (selectAllInput) {
      selectAllInput.checked = allChecked;
      selectAllInput.indeterminate = someChecked;
    }
  });

  onMount(() => {
    emitSelection();
  });

  let selectedCount = $derived(checked.filter(Boolean).length);
  let allChecked = $derived(result.candidates.length > 0 && selectedCount === result.candidates.length);
  let someChecked = $derived(selectedCount > 0 && !allChecked);
  let hasPreselection = $derived(preselectedNames.length > 0);
  let matchedPreselection = $derived(
    !hasPreselection || result.candidates.some(matchesPreselection)
  );

  function emitSelection() {
    onselectionchange(result.candidates.filter((_, index) => checked[index]));
  }

  function toggleAll(event: Event) {
    const newState = (event.currentTarget as HTMLInputElement).checked;
    checked = result.candidates.map(() => newState);
    emitSelection();
  }

  function toggleCandidate(index: number, event: Event) {
    const nextChecked = checked.slice();
    nextChecked[index] = (event.currentTarget as HTMLInputElement).checked;
    checked = nextChecked;
    emitSelection();
  }

  function getResult(candidate: SkillCandidate): InstallGithubResult | null {
    return installResults.find((item) => item?.candidate_name === candidate.name) ?? null;
  }
</script>

<div class="skill-select">
  <div class="results-header">
    <span class="repo-label">
      📁 <strong>{result.owner}/{result.repo}</strong>
      <span class="badge">{result.candidates.length} skill{result.candidates.length !== 1 ? 's' : ''} found</span>
    </span>

    <label class="toggle-all">
      <input
        bind:this={selectAllInput}
        type="checkbox"
        checked={allChecked}
        indeterminate={someChecked}
        onchange={(event) => toggleAll(event)}
        disabled={installing}
      />
      Select all
    </label>
  </div>

  {#if hasPreselection && !matchedPreselection}
    <p class="selection-warning">
      No GitHub candidate matched the queued skill name. Select the intended skill manually.
    </p>
  {/if}

  <ul class="candidates">
    {#each result.candidates as candidate, i}
      {@const res = getResult(candidate)}
      <li class="candidate" class:installed={res?.success} class:failed={res && !res.success}>
        <label class="candidate-label">
          <input
            bind:this={candidateInputs[i]}
            type="checkbox"
            checked={checked[i] ?? false}
            onchange={(event) => toggleCandidate(i, event)}
            disabled={installing || res !== null}
          />
          <span class="name">{candidate.name}</span>
          <span class="file-count">({candidate.files.length} file{candidate.files.length !== 1 ? 's' : ''})</span>
        </label>

        {#if res}
          {#if res.success}
            <span class="status ok">
              ✓ installed
              {#if res.skill_name !== res.original_name}
                as <code>{res.skill_name}</code>
                <span class="rename-note">({res.original_name} already existed)</span>
              {/if}
            </span>
          {:else}
            <span class="status error">✗ {res.error}</span>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>

  <p class="selection-count">{selectedCount} selected</p>
</div>

<style>
  .skill-select {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    padding: 1rem;
    margin-top: 0.5rem;
  }

  .results-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .repo-label {
    font-size: 0.9rem;
    color: var(--text-primary, #333);
  }

  .badge {
    font-size: 0.75rem;
    background: var(--accent, #4a9eff);
    color: white;
    padding: 0.15em 0.5em;
    border-radius: 99px;
    margin-left: 0.5rem;
  }

  .toggle-all {
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
    color: var(--text-secondary, #666);
  }

  .candidates {
    list-style: none;
    padding: 0;
    margin: 0 0 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .candidate {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    padding: 0.4rem 0.5rem;
    border-radius: 5px;
    flex-wrap: wrap;
  }

  .candidate.installed {
    background: var(--success-bg, rgba(34, 197, 94, 0.08));
  }

  .candidate.failed {
    background: var(--error-bg, rgba(239, 68, 68, 0.08));
  }

  .candidate-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
    flex: 1;
  }

  .name {
    font-weight: 500;
    font-size: 0.9rem;
  }

  .file-count,
  .selection-count {
    font-size: 0.8rem;
    color: var(--text-secondary, #888);
  }

  .selection-count {
    margin: 0;
  }

  .selection-warning {
    margin: 0 0 0.75rem;
    padding: 0.5rem 0.6rem;
    border-radius: 5px;
    color: var(--error, #ef4444);
    background: var(--error-bg, rgba(239, 68, 68, 0.08));
    font-size: 0.82rem;
  }

  .status {
    font-size: 0.8rem;
  }

  .status.ok {
    color: var(--success, #22c55e);
  }

  .status.error {
    color: var(--error, #ef4444);
    word-break: break-word;
  }

  .rename-note {
    color: var(--text-secondary, #888);
    font-style: italic;
  }

  code {
    font-size: 0.85em;
    background: var(--code-bg, rgba(0,0,0,0.08));
    padding: 0.1em 0.3em;
    border-radius: 3px;
  }
</style>
