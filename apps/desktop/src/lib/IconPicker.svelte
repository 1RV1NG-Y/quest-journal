<script lang="ts">
  import { tick } from 'svelte';
  import { Search, ChevronDown } from '@lucide/svelte';
  import QuestIcon from './QuestIcon.svelte';
  import { questIcons, questIconLabel } from './quest-icons';
  export let value = '';
  export let disabled = false;
  let trigger: HTMLButtonElement;
  async function choose(id: string) { value = id; expanded = false; await tick(); trigger.focus(); }
  let expanded = false;
  let search = '';
  let category = 'All';
  $: matches = questIcons.filter(icon => (category === 'All' || icon.category === category) && `${icon.label} ${icon.keywords}`.toLowerCase().includes(search.trim().toLowerCase()));
</script>
<div class="icon-picker">
  <span class="label" id="icon-picker-label">Quest icon</span>
  <div class="selection"><button class="choose" bind:this={trigger} type="button" {disabled} aria-labelledby="icon-picker-label icon-picker-value" aria-expanded={expanded} on:click={() => expanded = !expanded}><QuestIcon icon={value} size={24} /><span id="icon-picker-value">{questIconLabel(value)}</span><ChevronDown size={14} /></button>{#if value}<button class="text-action" type="button" {disabled} on:click={() => value = ''}>Reset</button>{/if}</div>
  {#if expanded}
    <div class="picker-options">
      <div class="search"><Search size={14} /><input type="search" aria-label="Search quest icons" placeholder="Search icons: Java, .NET, study…" bind:value={search} {disabled} on:keydown={(event) => { if (event.key === 'Enter') event.preventDefault(); if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); expanded = false; trigger.focus(); } }} /></div>
      <div class="categories" aria-label="Icon categories">{#each ['All','General','Tech'] as item}<button type="button" {disabled} aria-pressed={category === item} on:click={() => category = item}>{item}</button>{/each}</div>
      <div class="icon-grid" aria-label="Available quest icons">{#each matches as item}<button type="button" title={item.label} aria-label={`Use ${item.label} icon`} aria-pressed={value === item.id} {disabled} on:click={() => choose(item.id)}><QuestIcon icon={item.id} size={25} /><span>{item.label}</span></button>{/each}{#if !matches.length}<p>No matching icons. Try another name or category.</p>{/if}</div>
      <p class="credit">General icons · Lucide &nbsp; Tech logos · Devicon</p>
    </div>
  {/if}
</div>
<style>
  .label{display:block;font-size:13px;color:var(--text-soft);margin-bottom:8px}
  .selection{display:flex;gap:14px;align-items:center}.choose{display:flex;align-items:center;gap:12px;min-height:42px;padding:7px 12px;border:1px solid var(--border-strong);border-radius:6px;background:var(--surface-subtle);color:var(--text);cursor:pointer;font-size:13px}.choose span{min-width:95px;text-align:left}
  .picker-options{margin-top:10px;padding:12px;border:1px solid var(--border-strong);border-radius:6px;background:#181818}
  .search{display:flex;align-items:center;gap:8px;color:var(--text-muted)}.search input{min-height:34px;font-size:12px}
  .categories{display:flex;gap:7px;margin:10px 0}.categories button{border:0;border-radius:4px;padding:5px 10px;font-size:12px;background:transparent;color:var(--text-muted);cursor:pointer}.categories button[aria-pressed="true"]{background:#333;color:var(--text)}
  .icon-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(66px,1fr));gap:5px;max-height:220px;overflow:auto;padding:3px}
  .icon-grid button{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:7px;min-height:64px;min-width:0;padding:7px 4px;border:1px solid transparent;border-radius:5px;color:var(--text-soft);background:transparent;cursor:pointer}.icon-grid button:hover{background:#292929}.icon-grid button[aria-pressed="true"]{border-color:#888;background:#303030}.icon-grid button span{font-size:10px;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.icon-grid p{grid-column:1/-1;font-size:12px;color:var(--text-muted)}
  .credit{font-size:10px;color:var(--text-faint);margin:10px 0 0}
</style>
