<script lang="ts">
  /**
   * GithubInputRow — single URL input row in the link-input stack.
   * Triggers search when a valid GitHub URL is typed, shows per-row status.
   */
  import type { SearchResult } from '$lib/types';
  import { searchGithub } from '$lib/api';
  import { session } from '$lib/stores.svelte';
  import SearchResults from './SearchResults.svelte';

  // Svelte action: focus the element on mount if `shouldFocus` is true.
  // This avoids the a11y_autofocus lint warning from the `autofocus` HTML attribute.
  function focusOnMount(node: HTMLElement, shouldFocus: boolean) {
    if (shouldFocus) {
      // Defer to next microtask so the element is fully mounted
      Promise.resolve().then(() => node.focus());
    }
    return {};
  }

  interface Props {
    id: number;
    value: string;
    onchange: (id: number, value: string) => void;
    oninstalled: () => void;
    autofocus?: boolean;
  }

  let { id, value, onchange, oninstalled, autofocus = false }: Props = $props();

  type Status = 'idle' | 'searching' | 'done' | 'error';

  let status = $state<Status>('idle');
  let error = $state<string | null>(null);
  let result = $state<SearchResult | null>(null);
  let searchedUrl = $state<string>(''); // the URL that produced the current result

  function isGithubUrl(v: string): boolean {
    const t = v.trim().toLowerCase();
    return (
      t.startsWith('https://github.com/') ||
      t.startsWith('http://github.com/') ||
      /^[\w.-]+\/[\w.-]/.test(t)
    );
  }

  function handleInput(e: Event) {
    const newValue = (e.target as HTMLInputElement).value;
    onchange(id, newValue);

    // If cleared, reset state
    if (!newValue.trim()) {
      status = 'idle';
      error = null;
      result = null;
      searchedUrl = '';
    }
  }

  async function triggerSearch(url: string) {
    if (!url) return;
    if (status === 'searching') return;
    status = 'searching';
    error = null;
    result = null;
    searchedUrl = url;

    try {
      const res = await searchGithub(url, session.activeAgent);
      result = res;
      status = 'done';
    } catch (e) {
      error = String(e);
      status = 'error';
      result = null;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && value.trim() && isGithubUrl(value)) {
      triggerSearch(value.trim());
    }
  }
</script>

<div class="row-wrapper">
  <div class="input-row">
    <div class="input-container" class:searching={status === 'searching'}>
      <input
        type="text"
        class="url-input"
        placeholder="github.com/owner/repo  (or paste full URL)"
        {value}
        oninput={handleInput}
        onkeydown={handleKeydown}
        use:focusOnMount={autofocus}
        spellcheck={false}
        autocomplete="off"
      />
      {#if status === 'searching'}
        <span class="spinner" aria-label="Searching…">⟳</span>
      {/if}
    </div>
    {#if value.trim()}
      <button
        class="search-btn"
        onclick={() => triggerSearch(value.trim())}
        disabled={status === 'searching' || !isGithubUrl(value.trim())}
      >
        Search
      </button>
    {/if}
  </div>

  {#if status === 'error' && error}
    <p class="row-error">⚠ {error}</p>
  {/if}

  {#if status === 'done' && result}
    <SearchResults {result} {oninstalled} />
  {/if}
</div>

<style>
  .row-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .input-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .input-container {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .url-input {
    width: 100%;
    padding: 0.55rem 0.75rem;
    padding-right: 2rem;
    font-size: 0.9rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 6px;
    background: var(--input-bg, #fff);
    color: var(--text-primary, #222);
    font-family: 'Cascadia Code', 'JetBrains Mono', 'Fira Code', monospace;
    transition: border-color 0.15s;
    box-sizing: border-box;
  }

  .url-input:focus {
    outline: none;
    border-color: var(--accent, #4a9eff);
  }

  .input-container.searching .url-input {
    border-color: var(--accent, #4a9eff);
  }

  .spinner {
    position: absolute;
    right: 0.5rem;
    font-size: 1rem;
    animation: spin 1s linear infinite;
    color: var(--accent, #4a9eff);
    pointer-events: none;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .row-error {
    font-size: 0.82rem;
    color: var(--error, #ef4444);
    margin: 0;
    padding: 0 0.25rem;
  }

  .search-btn {
    padding: 0.55rem 1rem;
    font-size: 0.9rem;
    background: var(--btn-bg, #ebebeb);
    color: var(--text-primary, #1a1a1a);
    border: 1px solid var(--border-color, #ccc);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
    font-weight: 500;
  }

  .search-btn:hover:not(:disabled) {
    background: var(--btn-hover-bg, #d8d8d8);
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
