<script lang="ts">
  import { ExternalLink, FileText, Globe2 } from '@lucide/svelte';
  import type { ContinueResult, Resource, SavePoint } from './types';

  export let savePoint: SavePoint;
  export let restoring = false;
  export let error = '';
  export let result: ContinueResult | null = null;
  export let onrestore: (ids: string[]) => void;
  export let onclose: () => void;

  let selected = new Set(savePoint.resources.map((resource) => resource.id));

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id); else next.add(id);
    selected = next;
  }

  function metadata(resource: Resource): { title: string; detail: string } {
    if (resource.adapter_type === 'file') {
      const pieces = resource.resource_uri.split(/[\\/]/);
      return { title: pieces.at(-1) || resource.resource_uri, detail: resource.resource_uri };
    }
    try {
      const state = resource.state_json;
      const title = typeof state === 'object' && state !== null && 'title' in state && typeof state.title === 'string'
        ? state.title
        : undefined;
      const url = new URL(resource.resource_uri);
      return { title: title || url.hostname, detail: resource.resource_uri };
    } catch {
      return { title: resource.resource_uri, detail: resource.resource_uri };
    }
  }

  function outcome(id: string): 'success' | 'failure' | null {
    if (!result) return null;
    const item = result.find((entry) => entry.resource_id === id);
    return item ? (item.ok ? 'success' : 'failure') : null;
  }
</script>

<div class="continue-flow">
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  {#if result}
    <div class="notice success" role="status">Restore finished. Review the results below.</div>
  {/if}
  <div class="checkpoint"><span>Checkpoint</span><p>{savePoint.checkpoint || 'No checkpoint was recorded.'}</p></div>
  <fieldset disabled={restoring || !!result}>
    <legend>Select what to open</legend>
    {#if savePoint.resources.length === 0}
      <div class="resource-empty"><FileText size={20} /><p>This save point has no resources to reopen.</p></div>
    {:else}
      <div class="resource-list">
        {#each savePoint.resources as resource (resource.id)}
          {@const info = metadata(resource)}
          {@const status = outcome(resource.id)}
          <label class:selected={selected.has(resource.id)} class="resource-row">
            <input type="checkbox" checked={selected.has(resource.id)} on:change={() => toggle(resource.id)} />
            <span class="resource-icon">{#if resource.adapter_type === 'browser_tab'}<Globe2 size={17} />{:else}<FileText size={17} />{/if}</span>
            <span class="resource-copy"><strong>{info.title}</strong><small>{info.detail}</small></span>
            {#if status}<span class:failed={status === 'failure'} class="result">{status === 'success' ? 'Opened' : 'Could not open'}</span>{:else}<span class="open-mark"><ExternalLink size={15} /></span>{/if}
          </label>
        {/each}
      </div>
    {/if}
  </fieldset>
  <footer class="dialog-actions">
    <button class="button ghost" type="button" on:click={onclose}>{result ? 'Done' : 'Cancel'}</button>
    {#if !result}<button class="button primary" type="button" on:click={() => onrestore([...selected])} disabled={restoring || selected.size === 0}>{restoring ? 'Opening…' : `Open selected (${selected.size})`}</button>{/if}
  </footer>
</div>

<style>
  .continue-flow { display: grid; gap: var(--space-4); }
  .checkpoint { padding: var(--space-4); background: var(--surface-subtle); border-left: 2px solid var(--accent-blue); border-radius: 0 var(--radius-sm) var(--radius-sm) 0; }
  .checkpoint span, legend { color: var(--text-muted); font-size: var(--text-xs); font-weight: 700; letter-spacing: .09em; text-transform: uppercase; }
  .checkpoint p { margin: var(--space-2) 0 0; line-height: 1.5; }
  fieldset { min-width: 0; margin: 0; padding: 0; border: 0; }
  legend { margin-bottom: var(--space-2); }
  .resource-list { display: grid; gap: var(--space-2); }
  .resource-row { display: flex; align-items: center; gap: var(--space-3); min-width: 0; padding: var(--space-3); background: var(--surface-subtle); border: 1px solid var(--border); border-radius: var(--radius-md); cursor: pointer; transition: border-color var(--transition), background var(--transition); }
  .resource-row:hover, .resource-row.selected { background: var(--surface-hover); border-color: var(--border-strong); }
  .resource-icon { display: grid; flex: 0 0 2rem; height: 2rem; place-items: center; color: var(--accent-blue); background: var(--blue-muted); border-radius: var(--radius-sm); }
  .resource-copy { min-width: 0; flex: 1; }
  .resource-copy strong, .resource-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .resource-copy strong { font-size: var(--text-sm); font-weight: 600; }
  .resource-copy small { margin-top: .15rem; color: var(--text-muted); }
  .open-mark { color: var(--text-faint); }
  .result { flex: 0 0 auto; color: var(--success); font-size: var(--text-xs); font-weight: 700; }
  .result.failed { color: var(--danger); }
  .resource-empty { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-5); color: var(--text-muted); background: var(--surface-subtle); border: 1px dashed var(--border-strong); border-radius: var(--radius-md); }
  .resource-empty p { margin: 0; }
</style>
