<script lang="ts">
  import { onMount } from 'svelte';
  import { CirclePlus, FolderInput, Pencil, Trash2 } from '@lucide/svelte';
  export let x: number;
  export let y: number;
  export let title: string;
  export let hasChildren = false;
  export let onaction: (action: 'rename' | 'create' | 'move' | 'delete') => void;
  export let onclose: (restoreFocus?: boolean) => void;
  let menu: HTMLDivElement;
  onMount(() => {
    const rect = menu.getBoundingClientRect();
    x = Math.max(8, Math.min(x, window.innerWidth - rect.width - 8));
    y = Math.max(8, Math.min(y, window.innerHeight - rect.height - 8));
    menu.querySelector<HTMLButtonElement>('button')?.focus();
  });
  function keyboard(event: KeyboardEvent) {
    const items = [...menu.querySelectorAll<HTMLButtonElement>('button')];
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'Escape' || event.key === 'Tab') { event.preventDefault(); onclose(true); }
    else if (['ArrowDown','ArrowUp','Home','End'].includes(event.key)) {
      event.preventDefault();
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
      items[next].focus();
    }
  }
</script>
<svelte:window on:pointerdown={(event) => { if (menu && !menu.contains(event.target as Node)) onclose(false); }} on:resize={() => onclose(true)} />
<div class="quest-menu" role="menu" aria-label={`Actions for ${title}`} tabindex="-1" bind:this={menu} style={`left:${x}px;top:${y}px`} on:keydown={keyboard} on:contextmenu|preventDefault>
  <div class="menu-title" title={title}>{title}</div>
  <button role="menuitem" on:click={() => onaction('rename')}><Pencil size={14} /> Rename</button>
  <button role="menuitem" on:click={() => onaction('create')}><CirclePlus size={14} /> New subquest</button>
  <button role="menuitem" on:click={() => onaction('move')}><FolderInput size={14} /> Move to…</button>
  <div class="separator" role="separator"></div>
  <button class="delete" role="menuitem" on:click={() => onaction('delete')}><Trash2 size={14} /> {hasChildren ? 'Delete quest & subquests' : 'Delete quest'}</button>
</div>
<style>
  .quest-menu{position:fixed;z-index:1000;width:232px;padding:5px;background:#242424;border:1px solid #444;border-radius:7px;box-shadow:0 8px 28px #0008}
  .menu-title{font-size:11px;color:var(--text-faint);padding:7px 9px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  button{display:flex;align-items:center;gap:10px;width:100%;padding:9px;border:0;border-radius:4px;background:none;color:var(--text);font-size:12px;text-align:left;cursor:pointer}
  button:hover,button:focus-visible{background:#383838;box-shadow:none;outline:none}.delete{color:var(--danger)}
  .separator{border-top:1px solid #3b3b3b;margin:4px}
</style>
