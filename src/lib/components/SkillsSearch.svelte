<script lang="ts">
  import { searchSkills } from '$lib/api';
  import type { SkillsSearchItem } from '$lib/types';
  import BulkQueueInstall from './BulkQueueInstall.svelte';
  import CollapsibleSection from './CollapsibleSection.svelte';

  interface Props {
    oninstalled: () => void;
  }

  let { oninstalled }: Props = $props();

  type SearchStatus = 'idle' | 'searching' | 'done' | 'error';

  let query = $state('');
  let status = $state<SearchStatus>('idle');
  let error = $state<string | null>(null);
  let results = $state<SkillsSearchItem[]>([]);
  let queue = $state<SkillsSearchItem[]>([]);
  let submittedQueue = $state<SkillsSearchItem[]>([]);

  let queueCount = $derived(queue.length);

  function queueKey(item: SkillsSearchItem): string {
    return `${item.github_url}#${item.skill_slug}`;
  }

  function isQueued(item: SkillsSearchItem): boolean {
    return queue.some((queued) => queueKey(queued) === queueKey(item));
  }

  async function search() {
    const trimmed = query.trim();
    if (!trimmed || status === 'searching') return;

    status = 'searching';
    error = null;
    results = [];

    try {
      results = await searchSkills(trimmed);
      status = 'done';
    } catch (e) {
      status = 'error';
      error = String(e);
    }
  }

  function toggleQueue(item: SkillsSearchItem) {
    const key = queueKey(item);
    if (isQueued(item)) {
      queue = queue.filter((queued) => queueKey(queued) !== key);
    } else {
      queue = [...queue, item];
    }
  }

  function removeFromQueue(item: SkillsSearchItem) {
    const key = queueKey(item);
    queue = queue.filter((queued) => queueKey(queued) !== key);
  }

  function submitQueue() {
    if (queue.length === 0) return;
    submittedQueue = [...queue];
  }
</script>

<CollapsibleSection
  title="Find Skills"
  description="Search the skills.sh catalog and stage GitHub skills for installation."
>
  {#snippet actions()}
    {#if queueCount > 0}
      <span class="queue-badge">{queueCount} queued</span>
    {/if}
  {/snippet}

  <div class="skills-search">
    <form class="search-form" onsubmit={(event) => { event.preventDefault(); search(); }}>
      <input
        class="search-input"
        type="search"
        bind:value={query}
        placeholder="Find a skill, e.g. frontend or pdf"
        aria-label="Find skills"
        autocomplete="off"
      />
      <button class="search-btn" type="submit" disabled={status === 'searching' || !query.trim()}>
        {#if status === 'searching'}Searching…{:else}Search{/if}
      </button>
    </form>

    {#if status === 'error' && error}
      <p class="message error">⚠ {error}</p>
    {:else if status === 'done' && results.length === 0}
      <p class="message">No GitHub-backed skills matched that search.</p>
    {/if}

    {#if results.length > 0}
      <ul class="search-results">
        {#each results as item (queueKey(item))}
          {@const queued = isQueued(item)}
          <li class:queued>
            <button
              class="result-button"
              class:selected={queued}
              type="button"
              onclick={() => toggleQueue(item)}
              aria-pressed={queued}
            >
              <span class="result-main">
                <strong>{item.skill_name}</strong>
                <span class="github-url">{item.github_url}</span>
              </span>
              <span class="result-meta">
                <span>{item.installs.toLocaleString()} installs</span>
                <span>{queued ? '✓ queued' : '＋ queue'}</span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="queue">
      <div class="queue-heading">
        <div>
          <h3>Install queue</h3>
          <p>Click a result to add or remove it.</p>
        </div>
        {#if queue.length > 0}
          <button class="clear-btn" type="button" onclick={() => queue = []}>Clear</button>
        {/if}
      </div>

      {#if queue.length === 0}
        <p class="empty-queue">No skills queued yet.</p>
      {:else}
        <ul class="queue-list">
          {#each queue as item (queueKey(item))}
            <li>
              <div>
                <strong>{item.skill_name}</strong>
                <span>{item.github_url}</span>
              </div>
              <button class="remove-btn" type="button" onclick={() => removeFromQueue(item)} aria-label={`Remove ${item.skill_name}`}>
                Remove
              </button>
            </li>
          {/each}
        </ul>
        <button class="submit-queue" type="button" onclick={submitQueue}>
          {#if submittedQueue.length > 0}Refresh install panels{:else}Submit queue{/if} ({queueCount})
        </button>
      {/if}

    {#if submittedQueue.length > 0}
      <BulkQueueInstall skills={submittedQueue} {oninstalled} />
    {/if}
  </div>
</div>
</CollapsibleSection>

<style>
  .skills-search {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .queue-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
  }

  h3 {
    margin: 0;
  }

  h3 {
    font-size: 0.95rem;
  }

  .queue-heading p,
  .message,
  .empty-queue {
    margin: 0.15rem 0 0;
    color: var(--text-secondary, #777);
    font-size: 0.82rem;
  }

  .queue-badge {
    padding: 0.2rem 0.55rem;
    border-radius: 99px;
    background: var(--accent, #4a9eff);
    color: white;
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .search-form {
    display: flex;
    gap: 0.5rem;
  }

  .search-input {
    flex: 1;
    min-width: 0;
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 6px;
    background: var(--input-bg, #fff);
    color: var(--text-primary, #222);
    font-size: 0.9rem;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent, #4a9eff);
  }

  .search-btn,
  .submit-queue {
    padding: 0.6rem 1rem;
    border: none;
    border-radius: 6px;
    background: var(--accent, #4a9eff);
    color: white;
    cursor: pointer;
    font-size: 0.88rem;
  }

  .search-btn:disabled,
  .submit-queue:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .message.error {
    color: var(--error, #ef4444);
  }

  .search-results,
  .queue-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .result-button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 6px;
    background: var(--input-bg, #fff);
    color: var(--text-primary, #222);
    text-align: left;
    cursor: pointer;
  }

  .result-button:hover,
  .result-button.selected {
    border-color: var(--accent, #4a9eff);
    background: var(--hover-bg, rgba(0, 0, 0, 0.025));
  }

  .result-main,
  .result-meta,
  .queue-list li > div {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .result-main {
    min-width: 0;
  }

  .github-url,
  .queue-list li span {
    color: var(--text-secondary, #777);
    font-family: 'Cascadia Code', 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.76rem;
    overflow-wrap: anywhere;
  }

  .result-meta {
    align-items: flex-end;
    white-space: nowrap;
    color: var(--text-secondary, #777);
    font-size: 0.78rem;
  }

  .queue {
    padding: 0.8rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
  }

  .clear-btn,
  .remove-btn {
    border: none;
    background: transparent;
    color: var(--text-secondary, #777);
    cursor: pointer;
    font-size: 0.78rem;
  }

  .clear-btn:hover,
  .remove-btn:hover {
    color: var(--error, #ef4444);
  }

  .queue-list {
    margin-top: 0.75rem;
  }

  .queue-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border-color, #eee);
  }

  .queue-list li:last-child {
    border-bottom: none;
  }

  .submit-queue {
    margin-top: 0.75rem;
  }

  @media (max-width: 560px) {
    .search-form,
    .result-button {
      align-items: stretch;
      flex-direction: column;
    }

    .search-btn {
      width: 100%;
    }

    .result-meta {
      align-items: flex-start;
    }
  }
</style>
