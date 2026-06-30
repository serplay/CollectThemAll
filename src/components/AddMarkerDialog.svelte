<script lang="ts">
  // Modal for creating/editing a custom marker's title + notes. Presentational —
  // the parent (GameMapView) owns when it's open and what happens on save/delete.
  let {
    initialTitle = '',
    initialDescription = '',
    onSave,
    onDelete,
    onCancel,
  }: {
    initialTitle?: string;
    initialDescription?: string;
    onSave: (title: string, description: string) => void;
    onDelete?: () => void;
    onCancel: () => void;
  } = $props();

  let title = $state(initialTitle);
  let description = $state(initialDescription);

  function handleSave() {
    onSave(title.trim(), description.trim());
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel();
  }

  /** Tiny Svelte action: focuses the element as soon as it's mounted, replacing the
   *  `autofocus` attribute (which svelte-check flags as an a11y anti-pattern). */
  function focusOnMount(node: HTMLElement) {
    node.focus();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="dialog-backdrop" role="presentation" onclick={onCancel}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <h3>{initialTitle || initialDescription ? 'Edit marker' : 'Add marker'}</h3>

    <label class="field">
      <span>Title</span>
      <input type="text" bind:value={title} placeholder="e.g. Hidden chest" use:focusOnMount />
    </label>

    <label class="field">
      <span>Notes</span>
      <textarea bind:value={description} placeholder="Anything you want to remember…" rows="4"
      ></textarea>
    </label>

    <div class="dialog-actions">
      {#if onDelete}
        <button class="btn-delete" onclick={onDelete}>Delete</button>
      {/if}
      <div class="dialog-actions-right">
        <button class="btn-cancel" onclick={onCancel}>Cancel</button>
        <button class="btn-save" onclick={handleSave}>Save</button>
      </div>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .dialog {
    background: #161329;
    border: 1px solid rgba(167, 139, 250, 0.25);
    border-radius: 10px;
    padding: 1.25rem;
    width: 320px;
    max-width: calc(100vw - 2rem);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog h3 {
    margin: 0 0 1rem;
    font-size: 1rem;
    color: #f0ecff;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.85rem;
    font-size: 0.8rem;
    color: #c0b9c0;
  }

  .field input,
  .field textarea {
    background: #0d0b1e;
    border: 1px solid #3d3a4f;
    border-radius: 6px;
    padding: 0.45rem 0.6rem;
    color: #f0ecff;
    font-size: 0.85rem;
    font-family: inherit;
    resize: vertical;
  }

  .field input:focus,
  .field textarea:focus {
    outline: none;
    border-color: #a78bfa;
  }

  .dialog-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.5rem;
  }

  .dialog-actions-right {
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
  }

  button {
    border: none;
    border-radius: 6px;
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  button:hover {
    opacity: 0.85;
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid #3d3a4f;
    color: #c0b9c0;
  }

  .btn-save {
    background: linear-gradient(135deg, #7c3aed, #cf30aa);
    color: #fff;
  }

  .btn-delete {
    background: transparent;
    border: 1px solid #5b3a50;
    color: #f87171;
  }
</style>
