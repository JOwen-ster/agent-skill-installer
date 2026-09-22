<script lang="ts">
  /**
   * HistoryPanel — collapsible panel showing search + install history from SQLite.
   */
  import type { HistoryData } from '$lib/types';
  import { getHistory } from '$lib/api';
  import { session } from '$lib/stores.svelte';
  import CollapsibleSection from './CollapsibleSection.svelte';

  let open = $state(false);
  let rawHistory = $state<HistoryData | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  let history = $derived.by(() => {
    if (!rawHistory) return null;
    return {
      searches: rawHistory.searches.filter(s => s.agent === session.activeAgent),
      installs: rawHistory.installs.filter(i => i.agent === session.activeAgent)
    };
  });

  async function loadHistory() {
    loading = true;
    error = null;
    try {
      rawHistory = await getHistory();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleOpenChange(nextOpen: boolean) {
    open = nextOpen;
    if (nextOpen && !history) {
      loadHistory();
    }
  }

  // Export for parent to call after new installs
  export function refresh() {
    if (open) loadHistory();
  }

  function extractRepo(ref: string | null): string {
    if (!ref) return '';
    const match = ref.match(/github\.com\/([^/]+\/[^/]+)/);
    return match ? match[1] : '';
  }
</script>

<CollapsibleSection title="History" initialOpen={false} onopenchange={handleOpenChange}>
  {#snippet actions()}
    {#if history}
      <span class="counts">
        ({history.searches.length} search{history.searches.length !== 1 ? 'es' : ''},
        {history.installs.length} install{history.installs.length !== 1 ? 's' : ''})
      </span>
    {/if}
  {/snippet}

  <div class="history-content">
    {#if loading}
      <p class="loading">Loading history…</p>
    {:else if error}
      <p class="error">Failed to load history: {error}</p>
    {:else if history}
      <div class="history-columns">
        <div class="history-col">
          <h4>Recent Searches</h4>
          {#if history.searches.length === 0}
            <p class="empty">No searches yet.</p>
          {:else}
            <ul class="history-list">
              {#each history.searches as s}
                <li>
                  <span class="repo">{s.owner}/{s.repo}</span>
                  <span class="branch">@{s.branch}</span>
                  <span class="date">{s.searched_at.slice(0, 10)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <div class="history-col">
          <h4>Install History</h4>
          {#if history.installs.length === 0}
            <p class="empty">No installs yet.</p>
          {:else}
            <ul class="history-list">
              {#each history.installs as inst}
                <li class:ok={inst.status === 'ok'} class:err={inst.status === 'error'}>
                  <span class="status-dot" class:ok={inst.status === 'ok'} class:err={inst.status === 'error'}>
                    {inst.status === 'ok' ? '✓' : '✗'}
                  </span>
                  <span class="skill">{inst.skill_name}</span>
                  {#if inst.source_type === 'github' && inst.source_ref}
                    <span class="src-type" title={inst.source_ref}>[github: {extractRepo(inst.source_ref)}]</span>
                  {:else}
                    <span class="src-type">[{inst.source_type}]</span>
                  {/if}
                  <span class="date">{inst.installed_at.slice(0, 10)}</span>
                  {#if inst.error_message}
                    <span class="err-msg">{inst.error_message}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</CollapsibleSection>

<style>
  .counts {
    font-size: 0.78rem;
    opacity: 0.7;
  }

  .history-content {
    margin-top: 0.75rem;
  }

  .history-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
  }

  .history-col h4 {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--text-secondary, #888);
    margin: 0 0 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .history-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .history-list li {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    font-size: 0.8rem;
    padding: 0.2rem 0.3rem;
    border-radius: 3px;
    flex-wrap: wrap;
  }

  .history-list li.err {
    background: var(--error-bg, rgba(239, 68, 68, 0.05));
  }

  .repo, .skill {
    font-weight: 500;
  }

  .branch, .src-type, .date {
    font-size: 0.75rem;
    color: var(--text-secondary, #aaa);
  }

  .status-dot.ok { color: var(--success, #22c55e); }
  .status-dot.err { color: var(--error, #ef4444); }

  .err-msg {
    font-size: 0.75rem;
    color: var(--error, #ef4444);
    word-break: break-word;
    width: 100%;
  }

  .empty {
    font-size: 0.8rem;
    color: var(--text-secondary, #aaa);
    font-style: italic;
    margin: 0;
  }

  .loading, .error {
    font-size: 0.82rem;
    color: var(--text-secondary, #aaa);
    margin: 0;
  }

  .error {
    color: var(--error, #ef4444);
  }

  @media (max-width: 600px) {
    .history-columns {
      grid-template-columns: 1fr;
    }
  }
</style>
