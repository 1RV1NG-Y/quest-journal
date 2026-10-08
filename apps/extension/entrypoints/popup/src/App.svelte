<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from 'wxt/browser';
  import { validatePreview, RELOAD_EXTENSION_MESSAGE } from '../../../utils/preview';
  import { reloadOutdatedBackground } from '../../../utils/update';
  import type { CaptureScope, PauseResultData, PopupRequest, PopupResponse, PreviewData, Quest } from '../../../utils/protocol';

  let quests: Quest[] = [];
  let selectedQuestId = '';
  let pickerOpen = false;
  let search = '';
  let creating = false;
  let newTitle = '';
  let nestNew = true;
  let windowId: number | null = null;
  let scope: CaptureScope = 'current_window';
  let checkpoint = '';
  let action: 'add' | 'save' | 'close' = 'add';
  let preview: PreviewData = { protocol_version: 4, tabs: [], windows: [], suggested_ids: [], filtered_count: 0, context_label: 'This window' };
  let selected = new Set<number>();
  let anchor: number | null = null;
  let loading = true;
  let refreshing = false;
  let busy = false;
  let error = '';
  let message = '';
  let closeErrors: string[] = [];
  let requestSequence = 0;
  $: destination = quests.find(quest => quest.id === selectedQuestId);
  $: matches = quests.filter(quest => questPath(quest).toLowerCase().includes(search.trim().toLowerCase()));
  function questPath(quest: Quest): string {
    const names = [quest.title];
    const seen = new Set([quest.id]);
    let parent = quests.find(item => item.id === quest.parent_id);
    while (parent && !seen.has(parent.id)) { seen.add(parent.id); names.unshift(parent.title); parent = quests.find(item => item.id === parent?.parent_id); }
    return names.join(' / ');
  }
  function site(url: string) { try { return new URL(url).hostname; } catch { return url; } }
  async function send<T>(request: PopupRequest): Promise<T> {
    const response = await browser.runtime.sendMessage(request) as PopupResponse<T>;
    if (!response.ok) throw new Error(response.error);
    return response.data;
  }
  async function remember(id: string) {
    selectedQuestId = id; pickerOpen = false;
    try { await browser.storage.local.set({ lastQuestId: id }); } catch { /* Capture remains usable if preference storage is unavailable. */ }
  }
  async function loadQuests() {
    const data = await send<Quest[]>({type:'extension_list_quests'});
    if (!Array.isArray(data)) throw new Error('Could not read your quests.');
    quests = data;
    if (!quests.some(item => item.id === selectedQuestId)) selectedQuestId = quests[0]?.id ?? '';
    if (!quests.length) creating = true;
  }
  async function refreshPreview(reset = false) {
    const sequence = ++requestSequence;
    refreshing = true;
    try {
      if (windowId === null) throw new Error('Reopen the extension in the window you want to capture.');
      const data = validatePreview(await send<unknown>({type:'extension_preview',scope,window_id:windowId}));
      if (sequence !== requestSequence) return;
      preview = data;
      selected = reset ? new Set(data.suggested_ids.filter(id => data.tabs.some(tab => tab.id === id))) : new Set([...selected].filter(id => data.tabs.some(tab => tab.id === id)));
    } catch (cause) {
      if (sequence !== requestSequence) return;
      selected = new Set();
      preview = { protocol_version: 4, tabs: [], windows: [], suggested_ids: [], filtered_count: 0, context_label: 'This window' };
      error = cause instanceof Error ? cause.message : String(cause);
      if (error === RELOAD_EXTENSION_MESSAGE && !busy) {
        try {
          if (await reloadOutdatedBackground(browser)) {
            error = '';
            message = 'Finishing the extension update. Reopen this popup when it closes.';
          }
        } catch {
          // Keep the manual reload action available if preferences are inaccessible.
        }
      }
    }
    finally { if (sequence === requestSequence) refreshing = false; }
  }
  async function initialize() {
    loading = true; error = '';
    try {
      // Resolve in the popup context once; the worker's current window can be unrelated.
      if (windowId === null) {
        const current = await browser.windows.getCurrent();
        if (current.id === undefined || !Number.isInteger(current.id) || current.id < 0) throw new Error('Could not identify this window. Reopen the extension to try again.');
        windowId = current.id;
      }
      try { selectedQuestId = (await browser.storage.local.get('lastQuestId')).lastQuestId as string || ''; } catch { /* Optional preference. */ }
      await Promise.all([loadQuests(), refreshPreview(true)]);
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { loading = false; }
  }
  function toggle(id: number, shift: boolean) {
    const next = new Set(selected);
    const left = preview.tabs.findIndex(tab => tab.id === anchor);
    const right = preview.tabs.findIndex(tab => tab.id === id);
    if (shift && left >= 0) for (const tab of preview.tabs.slice(Math.min(left,right),Math.max(left,right)+1)) next.add(tab.id);
    else { next.has(id) ? next.delete(id) : next.add(id); anchor = id; }
    selected = next;
  }
  function selectWindow(id: number, checked: boolean) {
    const next = new Set(selected);
    for (const tab of preview.tabs.filter(tab => tab.window_id === id)) checked ? next.add(tab.id) : next.delete(tab.id);
    selected = next;
  }
  async function createQuest() {
    if (busy || newTitle.trim().length < 2) return;
    busy = true; error = '';
    try {
      const created = await send<Quest>({type:'extension_create_quest',title:newTitle.trim(),parent_id:nestNew ? selectedQuestId || null : null});
      quests = [...quests,created]; await remember(created.id); creating = false; newTitle = '';
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
  async function capture() {
    if (windowId === null || busy || refreshing || !selected.size || !selectedQuestId) return;
    busy = true; error = ''; message = ''; closeErrors = [];
    try {
      const result = await send<PauseResultData>({type:'extension_pause_quest',action,quest_id:selectedQuestId,checkpoint:checkpoint.trim(),scope,window_id:windowId,selected_tab_ids:[...selected],expected_urls:Object.fromEntries(preview.tabs.filter(tab => selected.has(tab.id)).map(tab => [String(tab.id),tab.url])),close_after_save:action === 'close'});
      message = `${result.captured_count} ${result.captured_count === 1 ? 'tab' : 'tabs'} ${action === 'add' ? 'added' : 'saved'} to ${destination ? questPath(destination) : 'quest'}.${action === 'close' ? ` ${result.closed_count} closed.` : ''}`;
      closeErrors = result.close_errors;
      selected = new Set(); checkpoint = ''; anchor = null;
      await remember(selectedQuestId);
      await refreshPreview();
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
  onMount(() => { void initialize(); });
</script>

<svelte:head><meta name="color-scheme" content="dark" /></svelte:head>
<main>
  <header><span>Quest Journal</span><button class="icon" title="Refresh quests and tabs" aria-label="Refresh quests and tabs" disabled={busy || loading || refreshing} on:click={initialize}>↻</button></header>
  {#if error}<div class="notice error" role="alert">{error}<button class="text" disabled={busy} on:click={initialize}>Retry connection / refresh</button><button class="text" disabled={busy} on:click={() => browser.runtime.reload()}>Reload extension</button></div>{/if}
  {#if message}<div class="notice" role="status">{message}{#each closeErrors as detail}<p>{detail}</p>{/each}</div>{/if}
  {#if loading}<p class="empty">Opening your quests…</p>{:else}
    <section class="destination">
      <div class="section-heading"><label for="destination">Destination</label><button class="text" disabled={busy} on:click={() => { creating = !creating; pickerOpen = false; }}>+ New quest</button></div>
      <button id="destination" class="destination-button" aria-expanded={pickerOpen} disabled={busy} on:click={() => pickerOpen = !pickerOpen}><span>{destination ? questPath(destination) : 'Choose a quest'}</span><span>⌄</span></button>
      {#if pickerOpen}<div class="quest-picker"><input type="search" placeholder="Find a quest…" aria-label="Find a destination quest" bind:value={search} /><div class="quest-list">{#each matches as quest}<button class:chosen={quest.id === selectedQuestId} title={questPath(quest)} disabled={busy} on:click={() => remember(quest.id)}><span>{quest.parent_id ? '↳ ' : ''}{quest.title}</span>{#if quest.parent_id}<small>{questPath(quest)}</small>{:else}<small>Top-level quest</small>{/if}</button>{/each}{#if !matches.length}<p class="empty">No matching quests.</p>{/if}</div></div>{/if}
      {#if creating}<form class="new-quest" on:submit|preventDefault={createQuest}><input aria-label="New quest name" placeholder="Quest name" bind:value={newTitle} maxlength="120" disabled={busy} />{#if destination}<label class="check"><input type="checkbox" bind:checked={nestNew} disabled={busy} /> Inside {questPath(destination)}</label>{/if}<div class="actions"><button class="secondary" disabled={busy || newTitle.trim().length < 2}>Create quest</button><button class="text" type="button" disabled={busy} on:click={() => creating = false}>Cancel</button></div></form>{/if}
    </section>
    <section>
      <div class="section-heading"><h2>Browser tabs</h2><span>{selected.size} selected</span></div>
      <div class="scope"><button class:chosen={scope === 'current_window'} disabled={busy || refreshing} on:click={() => { scope = 'current_window'; void refreshPreview(); }}>This window</button><button class:chosen={scope === 'all_windows'} disabled={busy || refreshing} on:click={() => { scope = 'all_windows'; void refreshPreview(); }}>All windows</button></div>
      <div class="selection-tools"><button class="text" disabled={busy || refreshing} on:click={() => selected = new Set(preview.tabs.map(tab => tab.id))}>Select all</button><button class="text" disabled={busy || refreshing} on:click={() => selected = new Set()}>Clear</button>{#if refreshing}<span>Refreshing…</span>{/if}</div>
      <div class="tabs">{#each preview.windows as window}<div class="window-group"><label class="window-label"><input type="checkbox" aria-label={`Select tabs in ${window.label}`} checked={preview.tabs.some(tab => tab.window_id === window.id) && preview.tabs.filter(tab => tab.window_id === window.id).every(tab => selected.has(tab.id))} disabled={busy || refreshing} on:change={(event) => selectWindow(window.id,event.currentTarget.checked)} /><span title={window.label}>{window.label}</span></label>{#each preview.tabs.filter(tab => tab.window_id === window.id) as tab (tab.id)}<label class="tab" class:selected={selected.has(tab.id)} title={tab.url}><input type="checkbox" checked={selected.has(tab.id)} disabled={busy || refreshing} on:click={(event) => toggle(tab.id,event.shiftKey)} /><span><strong>{tab.title}</strong><small>{site(tab.url)}{tab.pinned ? ' · Pinned' : ''}</small></span></label>{/each}</div>{/each}{#if !preview.tabs.length}<p class="empty">No supported tabs in this window.</p>{/if}</div>
      {#if preview.filtered_count}<p class="hint">{preview.filtered_count} browser or extension pages excluded.</p>{/if}
    </section>
    <section class="capture-actions"><label for="action">Action</label><select id="action" bind:value={action} disabled={busy}><option value="add">Add tabs</option><option value="save">Save session</option><option value="close">Save & close</option></select><p class="hint">{action === 'add' ? 'Add to the quest’s collection. Keep tabs open and leave its checkpoint unchanged.' : action === 'save' ? 'Save selected tabs and a checkpoint. Keep tabs open.' : 'Save selected tabs and a checkpoint, then close only the saved tabs.'} Existing materials stay in the quest.</p>
      {#if action !== 'add'}<label for="checkpoint">Where did you leave off? (optional)</label><textarea id="checkpoint" rows="2" maxlength="1000" placeholder="A short note for next time" bind:value={checkpoint} disabled={busy}></textarea>{/if}
      <button class="primary" disabled={busy || refreshing || !selected.size || !selectedQuestId || creating} on:click={capture}>{busy ? 'Saving…' : `${action === 'add' ? 'Add' : action === 'close' ? 'Save & close' : 'Save'} ${selected.size} ${selected.size === 1 ? 'tab' : 'tabs'}`}</button>
    </section>
  {/if}
</main>
