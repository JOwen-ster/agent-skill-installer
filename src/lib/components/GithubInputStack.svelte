<script lang="ts">
  /**
   * GithubInputStack — dynamic list of GithubInputRows.
   *
   * Invariants:
   *   - There is always exactly one empty trailing row.
   *   - Typing/pasting a GitHub URL into the last (empty) row appends a new empty row.
   *   - Clearing a non-trailing row back to empty removes that row.
   *   - The stack always has at least one row.
   */
  import GithubInputRow from './GithubInputRow.svelte';

  interface Props {
    oninstalled: () => void;
  }

  let { oninstalled }: Props = $props();

  interface Row {
    id: number;
    value: string;
  }

  let nextId = 0;
  function makeRow(value = ''): Row {
    return { id: nextId++, value };
  }

  let rows = $state<Row[]>([makeRow()]);

  function handleChange(id: number, newValue: string) {
    const idx = rows.findIndex((r) => r.id === id);
    if (idx === -1) return;

    // If a non-trailing row becomes empty, auto-remove it
    if (!newValue.trim() && idx !== rows.length - 1) {
      rows = rows.filter((r) => r.id !== id);
      return;
    }

    rows[idx] = { ...rows[idx], value: newValue };

    // If we just typed into the last row and it's not empty, append a new empty row
    if (idx === rows.length - 1 && newValue.trim()) {
      rows = [...rows, makeRow()];
    }
  }
</script>

<div class="input-stack">
  {#each rows as row, i (row.id)}
    <GithubInputRow
      id={row.id}
      value={row.value}
      onchange={handleChange}
      {oninstalled}
      autofocus={i === 0}
    />
  {/each}
</div>

<style>
  .input-stack {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
</style>
