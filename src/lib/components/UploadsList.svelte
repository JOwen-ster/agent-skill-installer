<script lang="ts">
  /**
   * UploadsList — shows drag-and-drop install results for this session (in-memory).
   */
  import type { DropInstallResult } from '$lib/types';

  interface Props {
    results: DropInstallResult[];
  }

  let { results }: Props = $props();
</script>

{#if results.length > 0}
  <section class="uploads-section">
    <h3 class="section-title">── Uploads this session ──</h3>
    <ul class="results-list">
      {#each results as r}
        <li class="result-item" class:ok={r.success} class:err={!r.success}>
          <span class="skill-name">
            {r.skill_name || r.path.split(/[/\\]/).pop()}
          </span>

          {#if r.success}
            <span class="status-badge ok">✓ installed</span>
            {#if r.skill_name !== r.original_name && r.original_name}
              <span class="rename-note">
                (renamed from <code>{r.original_name}</code>)
              </span>
            {/if}
          {:else}
            <span class="status-badge err">✗ failed</span>
            <span class="error-msg">{r.error}</span>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .uploads-section {
    margin-top: 0.5rem;
  }

  .section-title {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary, #888);
    margin: 0 0 0.5rem;
    letter-spacing: 0.02em;
  }

  .results-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .result-item {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    border-radius: 5px;
    font-size: 0.88rem;
    flex-wrap: wrap;
  }

  .result-item.ok {
    background: var(--success-bg, rgba(34, 197, 94, 0.07));
  }

  .result-item.err {
    background: var(--error-bg, rgba(239, 68, 68, 0.07));
  }

  .skill-name {
    font-weight: 500;
    min-width: 0;
    word-break: break-all;
  }

  .status-badge {
    font-size: 0.78rem;
    padding: 0.1em 0.45em;
    border-radius: 99px;
    flex-shrink: 0;
  }

  .status-badge.ok {
    background: var(--success, #22c55e);
    color: white;
  }

  .status-badge.err {
    background: var(--error, #ef4444);
    color: white;
  }

  .rename-note {
    font-size: 0.78rem;
    color: var(--text-secondary, #888);
    font-style: italic;
  }

  .error-msg {
    font-size: 0.78rem;
    color: var(--error, #ef4444);
    word-break: break-word;
  }

  code {
    font-size: 0.85em;
    background: var(--code-bg, rgba(0,0,0,0.08));
    padding: 0.1em 0.3em;
    border-radius: 3px;
  }
</style>
