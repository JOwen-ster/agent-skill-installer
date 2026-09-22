<script module lang="ts">
  let nextContentId = 0;
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    initialOpen?: boolean;
    actions?: Snippet;
    children: Snippet;
    onopenchange?: (open: boolean) => void;
  }

  let {
    title,
    description,
    initialOpen = true,
    actions,
    children,
    onopenchange = () => {},
  }: Props = $props();

  function getInitialOpen() {
    return initialOpen;
  }

  let open = $state(getInitialOpen());
  const contentId = `collapsible-section-content-${nextContentId++}`;

  function toggle() {
    open = !open;
    onopenchange(open);
  }
</script>

<section class="collapsible-section">
  <div class="section-heading">
    <div class="heading-copy">
      <button
        class="section-toggle"
        type="button"
        aria-expanded={open}
        aria-controls={contentId}
        onclick={toggle}
      >
        <span class="chevron" class:rotated={open} aria-hidden="true">▸</span>
        <span class="section-title" role="heading" aria-level="2">{title}</span>
      </button>
      {#if description}
        <p class="section-description">{description}</p>
      {/if}
    </div>

    {#if actions}
      <div class="section-actions">
        {@render actions()}
      </div>
    {/if}
  </div>

  <div id={contentId} class="section-content" hidden={!open} aria-hidden={!open}>
    {@render children()}
  </div>
</section>

<style>
  .collapsible-section {
    width: 100%;
  }

  .section-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .heading-copy {
    min-width: 0;
  }

  .section-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-primary, #222);
    cursor: pointer;
    text-align: left;
  }

  .section-toggle:hover {
    color: var(--accent, #4a9eff);
  }

  .section-toggle:focus-visible {
    outline: 2px solid var(--accent, #4a9eff);
    outline-offset: 3px;
    border-radius: 3px;
  }

  .chevron {
    display: inline-block;
    color: var(--text-secondary, #777);
    transition: transform 0.2s ease;
  }

  .chevron.rotated {
    transform: rotate(90deg);
  }

  .section-title {
    font-size: 1.05rem;
    font-weight: 600;
  }

  .section-description {
    margin: 0.15rem 0 0 1.25rem;
    color: var(--text-secondary, #777);
    font-size: 0.82rem;
  }

  .section-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .section-content[hidden] {
    display: none;
  }

  @media (max-width: 640px) {
    .section-heading {
      align-items: stretch;
      flex-direction: column;
    }

    .section-actions {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
