<script lang="ts">
  /**
   * ConfirmModal - A simple confirmation modal using the native <dialog> element.
   */
  interface Props {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { 
    title, 
    message, 
    confirmText = "Confirm", 
    cancelText = "Cancel",
    onconfirm, 
    oncancel 
  }: Props = $props();

  let dialog: HTMLDialogElement;

  export function show() {
    dialog.showModal();
  }

  export function close() {
    dialog.close();
  }

  function handleConfirm() {
    onconfirm();
    close();
  }

  function handleCancel() {
    oncancel();
    close();
  }

  function onDialogClick(e: MouseEvent) {
    // Close on backdrop click
    if (e.target === dialog) {
      handleCancel();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog bind:this={dialog} onclick={onDialogClick} onclose={oncancel}>
  <div class="modal-content">
    <h3 class="modal-title">{title}</h3>
    <p class="modal-message">{message}</p>
    
    <div class="modal-actions">
      <button class="btn btn-cancel" onclick={handleCancel}>{cancelText}</button>
      <button class="btn btn-confirm" onclick={handleConfirm}>{confirmText}</button>
    </div>
  </div>
</dialog>

<style>
  dialog {
    border: none;
    border-radius: 8px;
    padding: 0;
    max-width: 400px;
    width: 90%;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    background: var(--input-bg, #fff);
    color: var(--text-primary, #1a1a1a);
  }

  dialog::backdrop {
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
  }

  .modal-content {
    padding: 1.5rem;
  }

  .modal-title {
    margin: 0 0 0.75rem;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .modal-message {
    margin: 0 0 1.5rem;
    font-size: 0.95rem;
    color: var(--text-secondary, #666);
    line-height: 1.4;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .btn {
    appearance: none;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-cancel {
    background: var(--btn-bg, #ebebeb);
    color: var(--text-primary, #1a1a1a);
  }

  .btn-cancel:hover {
    background: var(--btn-hover-bg, #d8d8d8);
  }

  .btn-confirm {
    background: var(--error, #ef4444);
    color: white;
  }

  .btn-confirm:hover {
    background: #dc2626;
  }
</style>

