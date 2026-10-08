<script lang="ts">
  import QuestIcon from './QuestIcon.svelte';
  import { onDestroy, onMount, tick } from 'svelte';
  import { ArrowUpRight, FileText, FolderOpen, Globe, Shuffle, X } from '@lucide/svelte';
  import { buildShufflePool, pickShuffleItem, type ShuffleItem } from './shuffle';
  import { errorMessage } from './api';
  import type { Quest } from './types';

  export let quests: Quest[];
  export let currentQuest: Quest | null;
  export let onclose: () => void;
  export let onopen: (item: ShuffleItem) => Promise<void>;

  let scope = 'all';
  let topLevel = true;
  let subquests = true;
  let elements = false;
  let spinning = false;
  let opening = false;
  let result: ShuffleItem | null = null;
  let frames: ShuffleItem[] = [];
  let error = '';
  let notice = '';
  let heading: HTMLHeadingElement;
  let strip: HTMLDivElement;
  let animation: Animation | undefined;
  let destroyed = false;
  $: pool = buildShufflePool(quests, scope === 'current' ? currentQuest?.id ?? '__missing__' : null, { topLevel, subquests, elements });
  $: if (result && !pool.some(item => item.key === result?.key)) reset();

  onMount(() => heading.focus());
  onDestroy(() => { destroyed = true; animation?.cancel(); });

  function reset() {
    animation?.cancel(); animation = undefined;
    result = null; frames = []; error = ''; notice = '';
  }

  async function pick() {
    if (spinning || opening || !pool.length) return;
    reset();
    const winner = pickShuffleItem(pool)!;
    if (pool.length === 1 || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      result = winner;
      return;
    }
    spinning = true;
    frames = Array.from({ length: 26 }, () => pickShuffleItem(pool)!);
    frames.push(winner);
    await tick();
    if (destroyed) return;
    try {
      animation = strip.animate([
        { transform: 'translateY(0)' },
        { transform: `translateY(-${(frames.length - 1) * 72}px)` },
      ], { duration: 2400, easing: 'cubic-bezier(0.12, 0.62, 0.18, 1)', fill: 'forwards' });
      await animation.finished;
      if (!destroyed) result = winner;
    } catch { if (!destroyed) result = winner; }
    finally {
      if (!destroyed) { spinning = false; frames = []; animation?.cancel(); animation = undefined; }
    }
  }

  async function openResult() {
    if (!result || opening || spinning) return;
    opening = true; error = ''; notice = '';
    try {
      await onopen(result);
      if (!destroyed) notice = result.kind === 'tab' ? 'Opened in your browser.' : result.kind === 'file' ? 'Opened in its default app.' : '';
    } catch (cause) { if (!destroyed) error = errorMessage(cause); }
    finally { if (!destroyed) opening = false; }
  }
</script>

<svelte:window on:keydown={(event) => { if (event.key === 'Escape' && !opening) { event.preventDefault(); onclose(); } }} />

<section class="shuffle-workspace" aria-labelledby="shuffle-title">
  <header class="workspace-toolbar"><span><Shuffle size={15} /> Pick something</span><button class="icon-button" title="Back to quest" aria-label="Close picker" disabled={opening} on:click={onclose}><X size={16} /></button></header>
  <div class="picker">
    <h1 id="shuffle-title" bind:this={heading} tabindex="-1">Let a little chance decide.</h1>
    <p class="intro">Choose what goes in. See what comes up.</p>
    <div class="settings">
      <label for="shuffle-scope">Pick from</label>
      <select id="shuffle-scope" bind:value={scope} disabled={spinning || opening} on:change={reset}>
        <option value="all">All quests</option>
        <option value="current" disabled={!currentQuest}>This quest and everything inside{currentQuest ? ` · ${currentQuest.title}` : ''}</option>
      </select>
      <fieldset disabled={spinning || opening}>
        <legend>Include</legend>
        <label><input type="checkbox" bind:checked={topLevel} on:change={reset} /> Top-level quests</label>
        <label><input type="checkbox" bind:checked={subquests} on:change={reset} /> Subquests</label>
        <label><input type="checkbox" bind:checked={elements} on:change={reset} /> Elements</label>
      </fieldset>
      <div class="pool-summary"><span>{pool.length} {pool.length === 1 ? 'item' : 'items'} in the pool</span><span>{elements ? 'Elements include tabs and files' : 'Every item has equal odds'}</span></div>
    </div>
    <div class="reel" class:settled={!!result} aria-hidden={spinning || !!result}>
      {#if spinning}
        <div class="reel-strip" bind:this={strip}>{#each frames as item}<div class="reel-row"><strong>{item.title}</strong><small>{item.context}</small></div>{/each}</div>
      {:else if result}
        <div class="chosen-item">
          {#if result.kind === 'quest'}<QuestIcon icon={result.icon} size={28} />{:else if result.kind === 'tab'}<Globe size={22} />{:else}<FileText size={22} />{/if}
          <strong>{result.title}</strong><small>{result.context}</small>
        </div>
      {:else}
        <div class="reel-empty"><Shuffle size={25} /><span>{pool.length ? 'Your next possibility' : 'Nothing to pick yet'}</span><small>{pool.length ? 'A quest, a small step, a place to begin.' : !topLevel && !subquests && !elements ? 'Choose at least one type above.' : 'Try another scope or include more item types.'}</small></div>
      {/if}
    </div>
    <div class="announcement" role="status" aria-live="polite">{spinning ? 'Shuffling…' : result ? `Picked ${result.title}. ${result.context}` : ''}</div>
    {#if result?.location}<p class="location" title={result.location}>{result.location}</p>{/if}
    <div class="picker-actions">
      {#if result}<button class="button primary" disabled={opening} on:click={openResult}><ArrowUpRight size={15} /> {opening ? 'Opening…' : result.kind === 'quest' ? 'Open quest' : result.kind === 'tab' ? 'Open tab' : 'Open file'}</button>{/if}
      <button class="button" class:primary={!result} class:secondary={!!result} disabled={!pool.length || spinning || opening} on:click={pick}><Shuffle size={15} /> {spinning ? 'Picking…' : result ? 'Pick again' : 'Pick something'}</button>
    </div>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if notice}<p class="feedback" role="status">{notice}</p>{/if}
  </div>
</section>

<style>
  .picker{max-width:620px;margin:0 auto;padding:42px 28px 32px}
  h1{font-size:24px;letter-spacing:-.025em;font-weight:550;margin:0;outline:none}
  .intro{font-size:13px;color:var(--text-muted);margin:10px 0 30px}
  .settings>label,legend{font-size:12px;color:var(--text-muted)}
  select{margin:8px 0 20px;font-size:13px;background:#202020;color:var(--text);color-scheme:dark}
  fieldset{border:0;padding:0;margin:0;display:flex;gap:10px;flex-wrap:wrap}
  legend{margin-bottom:9px;padding:0}
  fieldset label{display:flex;align-items:center;gap:8px;padding:8px 11px;border:1px solid var(--border);border-radius:6px;font-size:12px;color:var(--text-muted);cursor:pointer;background:#1c1c1c}
  fieldset label:has(input:checked){color:var(--text);background:#292929;border-color:#505050}
  fieldset:disabled label{cursor:default;opacity:.6}
  input{margin:0;accent-color:#ddd}
  .pool-summary{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;margin:14px 0 25px;color:var(--text-faint);font-size:11px}
  .reel{height:216px;overflow:hidden;position:relative;border-top:1px solid var(--border);border-bottom:1px solid var(--border);background:#191919}
  .reel:not(.settled)::after{content:'';pointer-events:none;position:absolute;inset:0;background:linear-gradient(#191919 0%,transparent 35%,transparent 65%,#191919 100%)}
  .reel-strip{padding-top:72px;will-change:transform}
  .reel-row{height:72px;display:flex;flex-direction:column;align-items:center;justify-content:center;padding:0 24px;box-sizing:border-box;gap:7px}
  .reel-row strong{font-size:19px;font-weight:500;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .reel-row small{font-size:11px;color:var(--text-muted);max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .chosen-item,.reel-empty{height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;gap:12px;padding:20px 24px;box-sizing:border-box;color:var(--text-muted)}
  .chosen-item strong{color:var(--text);font-size:21px;font-weight:500;line-height:1.4;max-width:100%;overflow:auto;max-height:88px;overflow-wrap:anywhere}
  .chosen-item small{font-size:12px;max-height:44px;overflow:auto;overflow-wrap:anywhere}
  .reel-empty{font-size:15px}.reel-empty small{font-size:12px;color:var(--text-faint)}
  .settled{border-color:#525252;background:#202020}
  .location{font-size:11px;color:var(--text-faint);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;text-align:center;margin:12px 0 0}
  .picker-actions{display:flex;justify-content:center;gap:10px;margin-top:24px}
  .picker-actions button{min-height:35px;padding:0 15px}
  .feedback{font-size:12px;color:var(--text-muted);text-align:center}
  .announcement{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap}
  @media(max-width:700px){.picker{padding:28px 18px}.pool-summary{gap:7px}h1{font-size:21px}}
</style>
