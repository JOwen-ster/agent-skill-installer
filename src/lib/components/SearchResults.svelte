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
  let progress = $state('');
  let error = $state<string | null>(null);

  $effect(() => {
    selectedCandidates = result.candidates;
    installResults = result.candidates.map(() => null);
  });

  let pendingCandidates = $derived(selectedCandidates.filter((candidate) =>
    !installResults.some((installed) => installed?.candidate_name === candidate.name)
  ));
  let selectedCount = $derived(pendingCandidates.length);
  let hasInstallResults = $derived(installResults.some((item) => item !== null));

  function handleSelection(candidates: SkillCandidate[]) {
    selectedCandidates = candidates;
  }

  async function installSelected() {
    const requests: InstallGithubRequest[] = pendingCandidates.map((candidate) => ({
      owner: result.owner,
      repo: result.repo,
      branch: result.branch,
      candidate,
    }));

    if (requests.length === 0 || installing) return;

    installing = true;
    progress = `0 of ${requests.length} complete`;
    error = null;
    try {
      await installGithubSkills(requests, session.activeAgent, (installed, index) => {
        installResults = result.candidates.map((candidate, i) =>
          candidate.name === installed.candidate_name ? installed : installResults[i] ?? null
        );
        progress = `${index + 1} of ${requests.length} complete`;
      });
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
      oninstalled();
    }
  }
</script>

<SkillSelect
  {result}
  {installResults}
  {installing}
  onselectionchange={handleSelection}
/>

{#if installing}
  <p role="status">Installing… {progress}</p>
{/if}
{#if error}
  <p role="alert">{error}</p>
{/if}

{#if !hasInstallResults || selectedCount > 0 || installing}
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
