<script lang="ts">
  import { Archive, FileText } from '@lucide/svelte';
  import type { Quest } from './types';

  export let quest: Quest;
  export let saving = false;
  export let error = '';
  export let onsave: (checkpoint: string) => void;
  export let oncancel: () => void;

  let checkpoint = quest.current_checkpoint ?? '';

  function save() {
    if (saving) return;
    onsave(checkpoint.trim());
  }
</script>

<form on:submit|preventDefault={save}>
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  <div class="save-summary">
    <div class="summary-icon"><Archive size={19} /></div>
    <div><strong>Local save point</strong><p>Current tabs and attached file references will be included. Files stay in their original locations.</p></div>
  </div>
  {#if quest.files.length}
    <div class="included"><FileText size={16} /><span>{quest.files.length} attached {quest.files.length === 1 ? 'file' : 'files'} included</span></div>
  {/if}
  <div class="field">
    <label for="pause-checkpoint">Where did you leave off? (optional)</label>
    <textarea id="pause-checkpoint" bind:value={checkpoint} maxlength="500" rows="4" placeholder="The next useful place to begin"></textarea>
  </div>
  <p class="fine-print">This saves your current tab collection and checkpoint without closing anything on your computer.</p>
  <footer class="dialog-actions">
    <button class="button ghost" type="button" on:click={oncancel} disabled={saving}>Cancel</button>
    <button class="button primary" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Pause quest'}</button>
  </footer>
</form>

<style>
  form { display: grid; gap: var(--space-4); }
  .save-summary { display: flex; gap: var(--space-3); padding: var(--space-4); background: var(--surface-subtle); border: 1px solid var(--border); border-radius: var(--radius-md); }
  .summary-icon { display: grid; flex: 0 0 2.25rem; height: 2.25rem; place-items: center; color: var(--accent); background: var(--accent-muted); border-radius: var(--radius-sm); }
  .save-summary strong { font-size: var(--text-sm); }
  .save-summary p, .fine-print { margin: var(--space-1) 0 0; color: var(--text-muted); font-size: var(--text-sm); line-height: 1.45; }
  .included { display: flex; align-items: center; gap: var(--space-2); color: var(--text-soft); font-size: var(--text-sm); }
</style>
