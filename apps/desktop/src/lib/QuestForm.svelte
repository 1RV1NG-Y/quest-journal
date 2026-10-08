<script lang="ts">
  import IconPicker from './IconPicker.svelte';
  import type { Quest, QuestDesignation, QuestInput, QuestState } from './types';

  export let quest: Quest | null = null;
  export let saving = false;
  export let error = '';
  export let onsubmit: (input: QuestInput) => void;
  export let oncancel: () => void;

  let icon = quest?.icon ?? '';
  let title = quest?.title ?? '';
  let territory = quest?.territory ?? '';
  let objective = quest?.objective ?? '';
  let designation: QuestDesignation = quest?.designation ?? 'side';
  let state: QuestState = quest?.state ?? 'active';
  let currentCheckpoint = quest?.current_checkpoint ?? '';
  let attempted = false;

  $: valid = title.trim().length >= 2;

  function submit() {
    attempted = true;
    if (!valid || saving) return;
    onsubmit({
      icon,
      title: title.trim(),
      territory: territory.trim(),
      objective: objective.trim(),
      designation,
      state,
      current_checkpoint: currentCheckpoint.trim()
    });
  }
</script>

<form on:submit|preventDefault={submit} novalidate>
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  <div class="field">
    <label for="quest-form-title">Title <span aria-hidden="true">*</span></label>
    <input id="quest-form-title" bind:value={title} maxlength="120" autocomplete="off" aria-invalid={attempted && title.trim().length < 2} />
    {#if attempted && title.trim().length < 2}<small class="field-error">Enter at least 2 characters.</small>{/if}
  </div>
  <IconPicker bind:value={icon} disabled={saving} />
  <div class="field">
    <label for="quest-form-objective">Description</label>
    <textarea id="quest-form-objective" bind:value={objective} maxlength="600" rows="3" placeholder="What does moving this forward mean?" ></textarea>
  </div>
  <div class="field">
    <label for="quest-form-checkpoint">Current checkpoint</label>
    <textarea id="quest-form-checkpoint" bind:value={currentCheckpoint} maxlength="500" rows="2" placeholder="Where will you pick this up next?"></textarea>
  </div>
  <div class="form-grid">
    <div class="field compact">
      <label for="quest-form-state">State</label>
      <select id="quest-form-state" bind:value={state}>
        <option value="active">Active</option>
        <option value="paused">Paused</option>
        <option value="dormant">Dormant</option>
        <option value="waiting">Waiting</option>
        <option value="completed">Completed</option>
        <option value="abandoned">Set aside</option>
      </select>
    </div>
  </div>
  <footer class="dialog-actions">
    <button class="button ghost" type="button" on:click={oncancel} disabled={saving}>Cancel</button>
    <button class="button primary" type="submit" disabled={saving}>{saving ? 'Saving…' : quest ? 'Save changes' : 'Create quest'}</button>
  </footer>
</form>

<style>
  form { display: grid; gap: var(--space-4); }
  .form-grid { display: grid; grid-template-columns: 1fr; gap: var(--space-4); }
  .compact { align-content: start; }
  @media (max-width: 540px) { .form-grid { grid-template-columns: 1fr; } }
</style>
