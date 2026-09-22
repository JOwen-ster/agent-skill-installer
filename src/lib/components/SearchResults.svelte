<script lang="ts">
  /**
   * SearchResults — manual GitHub search wrapper.
   * SkillSelect owns the reusable checkbox list; this component owns the
   * existing GitHub installation action.
   */
  import type { InstallGithubRequest, InstallGithubResult, SearchResult, SkillCandidate } from '$lib/types';
  import { installGithubSkills } from '$lib/api';
  import { session } from '$lib/stores.svelte';
  import SkillSelect from './SkillSelect.svelte';

  interface Props {
    result: SearchResult;
    oninstalled: () => void;
  }

  let { result, oninstalled }: Props = $props();

  let selectedCandidates = $state<SkillCandidate[]>([]);
  let installResults = $state<(InstallGithubResult | null)[]>([]);
  let installing = $state(false);

  $effect(() => {
    selectedCandidates = result.candidates;
    installResults = result.candidates.map(() => null);
  });

  let selectedCount = $derived(selectedCandidates.length);
  let hasInstallResults = $derived(installResults.some((item) => item !== null));

  function handleSelection(candidates: SkillCandidate[]) {
    selectedCandidates = candidates;
  }

  async function installSelected() {
    const requests: InstallGithubRequest[] = selectedCandidates.map((candidate) => ({
      owner: result.owner,
      repo: result.repo,
      branch: result.branch,
      candidate,
    }));

    if (requests.length === 0 || installing) return;

    installing = true;
    try {
      const results = await installGithubSkills(requests, session.activeAgent);
      const resultMap = new Map(results.map((item) => [item.candidate_name, item]));
      installResults = result.candidates.map((candidate) => resultMap.get(candidate.name) ?? null);
      oninstalled();
    } catch (e) {
      console.error('Install error:', e);
    } finally {
      installing = false;
    }
  }
</script>

<SkillSelect
  {result}
  {installResults}
  {installing}
  onselectionchange={handleSelection}
/>

{#if !hasInstallResults}
  <button
    class="install-btn"
    onclick={installSelected}
    disabled={installing || selectedCount === 0}
  >
    {#if installing}
      Installing…
    {:else}
      Install selected ({selectedCount})
    {/if}
  </button>
{/if}

<style>
  .install-btn {
    margin-top: 0.6rem;
    padding: 0.5rem 1.25rem;
    font-size: 0.9rem;
    background: var(--accent, #4a9eff);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .install-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .install-btn:hover:not(:disabled) {
    opacity: 0.85;
  }
</style>
