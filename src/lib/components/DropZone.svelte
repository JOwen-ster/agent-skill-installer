<script lang="ts">
  /**
   * DropZone — accepts drag-and-drop of skill folders / SKILL.md files.
   * Listens to Tauri window drag events.
   */
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { DropInstallResult } from '$lib/types';
  import { installDroppedPaths } from '$lib/api';
  import { session } from '$lib/stores.svelte';

  import { open } from '@tauri-apps/plugin-dialog';
  import { homeDir } from '@tauri-apps/api/path';

  interface Props {
    onresults: (results: DropInstallResult[]) => void;
  }

  let { onresults }: Props = $props();

  let isDragging = $state(false);
  let isInstalling = $state(false);

  let unlisten: (() => void) | null = null;
  let pickerDialog: HTMLDialogElement;

  onMount(async () => {
    const win = getCurrentWindow();
    unlisten = await win.onDragDropEvent(async (event) => {
      if (event.payload.type === 'over') {
        isDragging = true;
      } else if (event.payload.type === 'leave') {
        isDragging = false;
      } else if (event.payload.type === 'drop') {
        isDragging = false;
        const paths: string[] = event.payload.paths;
        if (paths.length === 0) return;
        await processPaths(paths);
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function processPaths(paths: string[]) {
    isInstalling = true;
    try {
      const results = await installDroppedPaths(paths, session.activeAgent);
      onresults(results);
    } catch (e) {
      console.error('Drop install error:', e);
    } finally {
      isInstalling = false;
    }
  }

  async function choosePaths(directory: boolean) {
    try {
      const home = await homeDir();
      const selected = await open({
        multiple: true,
        directory,
        defaultPath: home,
        filters: directory ? undefined : [{ name: 'Markdown files', extensions: ['md'] }],
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        if (paths.length > 0) {
          await processPaths(paths);
        }
      }
    } catch (e) {
      console.error('Dialog error:', e);
    }
  }

  function handleClick() {
    if (!isInstalling) pickerDialog.showModal();
  }

  function chooseFromDialog(directory: boolean) {
    pickerDialog.close();
    choosePaths(directory);
  }
</script>

<div
  class="dropzone"
  class:active={isDragging}
  class:installing={isInstalling}
  role="button"
  tabindex="0"
  onclick={handleClick}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleClick(); }}
  aria-label="Drag and Drop Skill Content"
>
  {#if isInstalling}
    <span class="icon">⚙️</span>
    <p>Installing…</p>
  {:else if isDragging}
    <span class="icon">📂</span>
    <p>Release to install</p>
  {:else}
    <span class="icon">📦</span>
    <p>Drag a <code>folder</code>, <code>SKILL.md</code>, or click to browse</p>
    <p class="hint">Folders must contain at least 1 <code>SKILL.md</code>.</p>
  {/if}
</div>

<dialog bind:this={pickerDialog} class="picker-dialog" aria-labelledby="picker-title">
  <div class="picker-content">
    <h3 id="picker-title">Add skills</h3>
    <p>Choose what you want to install.</p>
    <div class="picker-actions">
      <button class="picker-option" onclick={() => chooseFromDialog(true)}>
        <span aria-hidden="true">📁</span>
        <span>
          <strong>Select folders</strong>
          <small>Choose folders containing skill files</small>
        </span>
      </button>
      <button class="picker-option" onclick={() => chooseFromDialog(false)}>
        <span aria-hidden="true">📄</span>
        <span>
          <strong>Select files</strong>
          <small>Choose individual Markdown skill files</small>
        </span>
      </button>
    </div>
    <button class="cancel-btn" onclick={() => pickerDialog.close()}>Cancel</button>
  </div>
</dialog>

<style>
  .dropzone {
    border: 2px dashed var(--border-color, #aaa);
    border-radius: 10px;
    padding: 2rem 1.5rem;
    text-align: center;
    transition: border-color 0.15s, background 0.15s;
    cursor: pointer;
    user-select: none;
    background: var(--dropzone-bg, transparent);
  }

  .dropzone:hover:not(.active):not(.installing) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.025));
  }

  .dropzone.active {
    border-color: var(--accent, #4a9eff);
    background: var(--dropzone-active-bg, rgba(74, 158, 255, 0.08));
  }

  .dropzone.installing {
    border-color: var(--accent, #4a9eff);
    opacity: 0.8;
    cursor: default;
  }

  .icon {
    font-size: 2rem;
    display: block;
    margin-bottom: 0.5rem;
  }

  p {
    margin: 0.25rem 0;
    font-size: 0.95rem;
    color: var(--text-secondary, #666);
  }

  .hint {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  code {
    font-size: 0.85em;
    background: var(--code-bg, rgba(0,0,0,0.08));
    padding: 0.1em 0.3em;
    border-radius: 3px;
  }

  .picker-dialog {
    border: none;
    border-radius: 8px;
    padding: 0;
    width: min(380px, 90vw);
    background: var(--input-bg, #fff);
    color: var(--text-primary, #222);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .picker-dialog::backdrop {
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
  }

  .picker-content {
    padding: 1.5rem;
  }

  .picker-content h3 {
    margin: 0 0 0.4rem;
    font-size: 1.15rem;
  }

  .picker-content p {
    margin: 0 0 1rem;
  }

  .picker-actions {
    display: grid;
    gap: 0.6rem;
  }

  .picker-option {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 6px;
    background: var(--btn-bg, #ebebeb);
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .picker-option:hover {
    background: var(--btn-hover-bg, #d8d8d8);
    border-color: var(--accent, #4a9eff);
  }

  .picker-option > span:first-child {
    font-size: 1.4rem;
  }

  .picker-option strong,
  .picker-option small {
    display: block;
  }

  .picker-option small {
    margin-top: 0.1rem;
    color: var(--text-secondary, #666);
  }

  .cancel-btn {
    display: block;
    margin: 1rem 0 0 auto;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    background: var(--btn-bg, #ebebeb);
    color: var(--text-primary, #222);
    cursor: pointer;
  }

  .cancel-btn:hover {
    background: var(--btn-hover-bg, #d8d8d8);
  }
</style>
