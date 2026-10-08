<script lang="ts">
  import { X } from '@lucide/svelte';

  export let open = false;
  export let title: string;
  export let description = '';
  export let wide = false;
  export let onclose: () => void;

  let element: HTMLDialogElement;

  $: if (element) {
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }

  function close() {
    onclose();
  }

  function handleClose() {
    if (open) onclose();
  }
</script>

<dialog
  bind:this={element}
  class:wide
  aria-labelledby="dialog-title"
  aria-describedby={description ? 'dialog-description' : undefined}
  on:close={handleClose}
  on:cancel={(event) => { event.preventDefault(); close(); }}
  on:click={(event) => { if (event.target === element) close(); }}
>
  <section class="dialog-frame">
    <header>
      <div>
        <h2 id="dialog-title">{title}</h2>
        {#if description}<p id="dialog-description">{description}</p>{/if}
      </div>
      <button class="icon-button" type="button" aria-label="Close dialog" on:click={close}><X size={19} /></button>
    </header>
    <div class="dialog-body"><slot /></div>
  </section>
</dialog>

<style>
  dialog {
    width: min(34rem, calc(100vw - var(--space-4)));
    max-height: calc(100vh - var(--space-6));
    padding: 0;
    overflow: hidden;
    color: var(--text);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
  }
  dialog.wide { width: min(48rem, calc(100vw - var(--space-4))); }
  dialog::backdrop { background: var(--backdrop); }
  .dialog-frame { display: flex; max-height: calc(100vh - var(--space-6)); flex-direction: column; }
  header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-4); padding: var(--space-5) var(--space-5) var(--space-4); border-bottom: 1px solid var(--border); }
  h2 { margin: 0; font-size: var(--text-xl); letter-spacing: -0.02em; }
  p { margin: var(--space-1) 0 0; color: var(--text-muted); font-size: var(--text-sm); }
  .dialog-body { overflow-y: auto; padding: var(--space-5); }
  @media (max-width: 600px) {
    dialog, dialog.wide { width: calc(100vw - var(--space-2)); max-height: calc(100vh - var(--space-2)); }
    .dialog-frame { max-height: calc(100vh - var(--space-2)); }
    header, .dialog-body { padding: var(--space-4); }
  }
</style>
