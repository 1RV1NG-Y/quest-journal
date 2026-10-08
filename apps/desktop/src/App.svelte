<script lang="ts">
  import { onMount } from 'svelte';
  import { Archive, BookOpen, Check, ChevronRight, CirclePlus, Compass, FilePlus2, FileText, FolderOpen, Pause, Pencil, Play, RefreshCw, Search, Shuffle, Ellipsis, Trash2, Undo2, X } from '@lucide/svelte';
  import { journalApi, errorMessage } from './lib/api';
  import type { ContinueResult, Quest, QuestInput, QuestState, SavePoint, Material } from './lib/types';
  import Dialog from './lib/Dialog.svelte';
  import QuestIcon from './lib/QuestIcon.svelte';
  import QuestMenu from './lib/QuestMenu.svelte';
  import ShufflePanel from './lib/ShufflePanel.svelte';
  import type { ShuffleItem } from './lib/shuffle';
  import QuestForm from './lib/QuestForm.svelte';
  import PauseDialog from './lib/PauseDialog.svelte';
  import ContinueDialog from './lib/ContinueDialog.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Minus, Square, ArrowUp, ArrowDown } from '@lucide/svelte';

  let questMenu: { quest: Quest; x: number; y: number; trigger: HTMLElement } | null = null;
  let deletedQuests: Quest[] = [];
  let questTrashOpen = false;
  let questTrashBusy = false;
  let deletedNotice: { id: string; title: string } | null = null;
  let renameTarget: Quest | null = null;
  let renameTitle = '';
  let renameBusy = false;
  let renameError = '';
  function showQuestMenu(event: MouseEvent, quest: Quest, atButton = false) {
    event.preventDefault(); event.stopPropagation();
    if (materialBusy || orderBusy || questTrashBusy || formSaving) return;
    const trigger = event.currentTarget as HTMLElement;
    const rect = trigger.getBoundingClientRect();
    questMenu = { quest, trigger, x: atButton || (event.clientX === 0 && event.clientY === 0) ? rect.right : event.clientX, y: atButton || (event.clientX === 0 && event.clientY === 0) ? rect.bottom : event.clientY };
  }
  function closeQuestMenu(restoreFocus = false) {
    if (restoreFocus) questMenu?.trigger.focus();
    questMenu = null;
  }
  async function questMenuAction(action: 'rename' | 'create' | 'move' | 'delete') {
    const quest = questMenu?.quest;
    closeQuestMenu(true);
    if (!quest) return;
    if (action === 'create') { newQuest(quest.id); return; }
    if (action === 'delete') { await deleteQuest(quest); return; }
    if (action === 'rename') {
      renameTarget = quest; renameTitle = quest.title; renameError = ''; return;
    }
    await openQuest(quest);
    if (!pageError) { destination = quest.parent_id ?? ''; moving = true; }
  }
  async function renameQuest() {
    if (!renameTarget || renameBusy || renameTitle.trim().length < 2) return;
    renameBusy = true; renameError = '';
    try {
      const current = await journalApi.getQuest(renameTarget.id);
      setUpdated(await journalApi.updateQuest(current.id, { ...questInput(current, current.state), title: renameTitle.trim() }));
      renameTarget = null;
    } catch (error) { renameError = errorMessage(error); }
    finally { renameBusy = false; }
  }
  async function deleteQuest(quest: Quest) {
    if (questTrashBusy) return;
    questTrashBusy = true; pageError = '';
    try {
      await journalApi.trashQuest(quest.id);
      deletedNotice = { id: quest.id, title: quest.title };
      shuffleOpen = false;
      selectedMaterials = new Set(); selectionAnchor = null; materialNotice = ''; moving = false;
      await loadQuests();
    } catch (error) { pageError = errorMessage(error); }
    finally { questTrashBusy = false; }
  }
  async function restoreDeletedQuest(id: string) {
    if (questTrashBusy) return;
    questTrashBusy = true; pageError = '';
    try {
      await journalApi.restoreQuest(id);
      if (deletedNotice?.id === id) deletedNotice = null;
      await loadQuests();
    } catch (error) { pageError = errorMessage(error); }
    finally { questTrashBusy = false; }
  }

  let shuffleOpen = false;
  let shuffleTrigger: HTMLButtonElement;
  async function showShuffle() {
    await loadQuests();
    if (!pageError) { shuffleOpen = true; questTrashOpen = false; }
  }
  function closeShuffle() { shuffleOpen = false; shuffleTrigger?.focus(); }
  async function openShuffleItem(item: ShuffleItem) {
    if (item.kind === 'quest') {
      const quest = quests.find(quest => quest.id === item.questId);
      if (!quest) throw new Error('This quest is no longer available. Reopen the picker to refresh.');
      await openQuest(quest);
      if (pageError) throw new Error(pageError);
      return;
    }
    const outcomes = item.kind === 'tab'
      ? await journalApi.openMaterials(item.questId, [item.id])
      : [await journalApi.openQuestFile(item.questId, item.id)];
    const failures = outcomes.filter(outcome => !outcome.ok);
    if (failures.length) throw new Error(failures.map(outcome => outcome.error || 'Could not open this item.').join('; '));
  }

  let quests: Quest[] = [];
  let selectedQuest: Quest | null = null;
  let loading = true;
  let pageError = '';
  let actionError = '';
  let searchQuery = '';
  let formMode: 'create' | 'edit' | null = null;
  let formSaving = false;
  let pauseQuest: Quest | null = null;
  let pauseSaving = false;
  let continueQuest: Quest | null = null;
  let continuePreview: SavePoint | null = null;
  let previewLoading = false;
  let restoring = false;
  let restoreResult: ContinueResult | null = null;
  let stateChange: { quest: Quest; state: 'completed' | 'abandoned' } | null = null;
  let stateSaving = false;
  let fileBusy = false;

  let selectedMaterials = new Set<string>();
  let selectionAnchor: string | null = null;
  let materialDestination = '';
  let materialBusy = false;
  let materialNotice = '';
  function selectMaterial(id: string, shift: boolean) {
    const next = new Set(selectedMaterials);
    const anchor = browserMaterials.findIndex(item => item.id === selectionAnchor);
    const current = browserMaterials.findIndex(item => item.id === id);
    if (shift && anchor >= 0) {
      for (const item of browserMaterials.slice(Math.min(anchor,current), Math.max(anchor,current)+1)) next.add(item.id);
    } else { next.has(id) ? next.delete(id) : next.add(id); selectionAnchor = id; }
    selectedMaterials = next;
  }
  let showTrash = false;
  async function trashMaterials(ids: string[], trashed = true) {
    if (!selectedQuest || materialBusy) return;
    materialBusy = true; actionError = ''; materialNotice = '';
    try {
      await journalApi.setMaterialsTrashed(selectedQuest.id, ids, trashed);
      materialNotice = `${ids.length} ${ids.length === 1 ? 'tab' : 'tabs'} ${trashed ? 'moved to trash' : 'restored'}.`;
      selectedMaterials = new Set(); selectionAnchor = null;
      await loadQuests();
    } catch (error) { actionError = errorMessage(error); }
    finally { materialBusy = false; }
  }
  async function moveSelectedMaterials() {
    if (!selectedQuest || !materialDestination || materialBusy) return;
    if (materialDestination === '__trash__') { await trashMaterials([...selectedMaterials]); return; }
    materialBusy = true; actionError = ''; materialNotice = '';
    const count = selectedMaterials.size;
    try {
      await journalApi.moveMaterials(selectedQuest.id, materialDestination, browserMaterials.filter(item => selectedMaterials.has(item.id)).map(item => item.id));
      materialNotice = `Moved ${count} ${count === 1 ? 'tab' : 'tabs'} to ${quests.find(item => item.id === materialDestination)?.title}.`;
      selectedMaterials = new Set(); selectionAnchor = null;
      await loadQuests();
    } catch (error) { actionError = errorMessage(error); }
    finally { materialBusy = false; }
  }
  async function openMaterials(ids: string[]) {
    if (!selectedQuest || materialBusy) return;
    materialBusy = true; actionError = ''; materialNotice = '';
    try {
      const outcomes = await journalApi.openMaterials(selectedQuest.id, ids);
      const failures = outcomes.filter(item => !item.ok);
      if (failures.length) actionError = failures.map(item => item.error || 'Could not open tab').join('; ');
      else materialNotice = `Opened ${outcomes.length} ${outcomes.length === 1 ? 'tab' : 'tabs'}.`;
    } catch (error) { actionError = errorMessage(error); }
    finally { materialBusy = false; }
  }

  let draggedQuest: string | null = null;
  let dropTarget: string | null = null;
  let orderBusy = false;
  async function saveOrder(parentId: string | null, ids: string[]) {
    orderBusy = true; pageError = '';
    try { await journalApi.reorderQuests(parentId, ids); await loadQuests(); }
    catch (error) { pageError = errorMessage(error); }
    finally { orderBusy = false; draggedQuest = null; dropTarget = null; }
  }
  function canStep(quest: Quest, direction: number) {
    const siblings = quests.filter(item => item.parent_id === quest.parent_id);
    const index = siblings.findIndex(item => item.id === quest.id);
    return index + direction >= 0 && index + direction < siblings.length;
  }
  async function stepQuest(quest: Quest, direction: number) {
    if (orderBusy || !canStep(quest, direction)) return;
    const ids = quests.filter(item => item.parent_id === quest.parent_id).map(item => item.id);
    const index = ids.indexOf(quest.id);
    [ids[index],ids[index+direction]] = [ids[index+direction],ids[index]];
    await saveOrder(quest.parent_id, ids);
  }
  function validDrop(target: Quest) {
    const source = quests.find(item => item.id === draggedQuest);
    return !orderBusy && source && source.id !== target.id && source.parent_id === target.parent_id;
  }
  async function dropQuest(target: Quest, event: DragEvent) {
    if (!validDrop(target)) return;
    event.preventDefault();
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const after = event.clientY > bounds.top + bounds.height / 2;
    const ids = quests.filter(item => item.parent_id === target.parent_id && item.id !== draggedQuest).map(item => item.id);
    ids.splice(ids.indexOf(target.id) + (after ? 1 : 0), 0, draggedQuest!);
    await saveOrder(target.parent_id, ids);
  }

  let expanded = new Set<string>();
  let createParentId: string | null = null;
  let moving = false;
  let moveBusy = false;
  let destination = '';
  function ancestors(quest: Quest | null, items: Quest[]): Quest[] {
    const result: Quest[] = [];
    const seen = new Set<string>();
    while (quest && !seen.has(quest.id)) {
      seen.add(quest.id); result.unshift(quest);
      quest = items.find(item => item.id === quest?.parent_id) ?? null;
    }
    return result;
  }
  function treeRows(items: Quest[], opened: Set<string>, query: string) {
    const rows: { quest: Quest; depth: number; children: boolean }[] = [];
    const matches = new Set<string>();
    if (query) for (const item of items) {
      if (`${item.title} ${item.objective} ${item.current_checkpoint}`.toLowerCase().includes(query)) {
        for (const ancestor of ancestors(item, items)) matches.add(ancestor.id);
      }
    }
    function walk(parent: string | null, depth: number) {
      for (const quest of items.filter(item => item.parent_id === parent)) {
        if (query && !matches.has(quest.id)) continue;
        const children = items.some(item => item.parent_id === quest.id);
        rows.push({ quest, depth, children });
        if (opened.has(quest.id) || query) walk(quest.id, depth + 1);
      }
    }
    walk(null, 0); return rows;
  }
  $: rows = treeRows(quests, expanded, normalizedSearch);
  $: breadcrumbs = ancestors(selectedQuest, quests);
  $: children = quests.filter(quest => quest.parent_id === selectedQuest?.id);
  $: destinations = quests.filter(quest => !ancestors(quest, quests).some(item => item.id === selectedQuest?.id));
  function toggle(id: string) { const next = new Set(expanded); next.has(id) ? next.delete(id) : next.add(id); expanded = next; }
  function newQuest(parent: string | null = null) { createParentId = parent; actionError = ''; formMode = 'create'; }
  async function moveQuest() {
    if (!selectedQuest || moveBusy) return;
    moveBusy = true; actionError = '';
    try {
      const updated = await journalApi.moveQuest(selectedQuest.id, destination || null);
      setUpdated(updated);
      await loadQuests();
      expanded = new Set([...expanded, ...ancestors(updated, quests).map(item => item.id)]);
      moving = false;
    } catch (error) { actionError = errorMessage(error); }
    finally { moveBusy = false; }
  }

  $: normalizedSearch = searchQuery.trim().toLocaleLowerCase();
  $: browserMaterials = selectedQuest?.materials ?? [];
  function materialTitle(resource: Material) { const state = resource.state_json as { title?: string } | null; return state?.title || resource.resource_uri; }
  function site(uri: string) { try { return new URL(uri).hostname; } catch { return uri; } }
  async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close') { try { await getCurrentWindow()[action](); } catch (error) { pageError = errorMessage(error); } }

  onMount(loadQuests);

  async function loadQuests() {
    loading = true;
    pageError = '';
    try {
      [quests, deletedQuests] = await Promise.all([journalApi.listQuests(), journalApi.listTrashedQuests()]);
      selectedQuest = quests.find((quest) => quest.id === selectedQuest?.id) ?? quests[0] ?? null;
      selectedMaterials = new Set([...selectedMaterials].filter(id => selectedQuest?.materials.some(item => item.id === id)));
      expanded = new Set([...expanded, ...ancestors(selectedQuest, quests).slice(0, -1).map(item => item.id)]);
    } catch (error) { pageError = errorMessage(error); }
    finally { loading = false; }
  }

  async function openQuest(quest: Quest) {
    if (materialBusy) return;
    selectedMaterials = new Set(); selectionAnchor = null; materialDestination = ''; materialNotice = '';
    pageError = '';
    try { selectedQuest = await journalApi.getQuest(quest.id); shuffleOpen = false; questTrashOpen = false; moving = false; actionError = ''; expanded = new Set([...expanded, ...ancestors(selectedQuest, quests).slice(0, -1).map(item => item.id)]); }
    catch (error) { pageError = errorMessage(error); }
  }

  async function saveQuest(input: QuestInput) {
    formSaving = true;
    actionError = '';
    try {
      const saved = formMode === 'edit' && selectedQuest ? await journalApi.updateQuest(selectedQuest.id, input) : await journalApi.createQuest({ ...input, parent_id: createParentId });
      quests = formMode === 'edit' ? quests.map((quest) => quest.id === saved.id ? saved : quest) : [saved, ...quests];
      selectedQuest = saved; shuffleOpen = false; questTrashOpen = false;
      selectedMaterials = new Set(); selectionAnchor = null; materialNotice = '';
      expanded = new Set([...expanded, ...ancestors(saved, quests).map(item => item.id)]);
      formMode = null;
      await loadQuests();
    } catch (error) { actionError = errorMessage(error); }
    finally { formSaving = false; }
  }

  async function attachFiles() {
    if (!selectedQuest || fileBusy) return;
    actionError = '';
    try {
      const paths = await journalApi.chooseFiles();
      if (!paths.length) return;
      fileBusy = true;
      const updated = await journalApi.addQuestFiles(selectedQuest.id, paths);
      setUpdated(updated);
    } catch (error) { actionError = errorMessage(error); }
    finally { fileBusy = false; }
  }

  async function removeFile(id: string) {
    if (!selectedQuest || fileBusy) return;
    fileBusy = true;
    actionError = '';
    try {
      await journalApi.removeQuestFile(id);
      setUpdated(await journalApi.getQuest(selectedQuest.id));
    } catch (error) { actionError = errorMessage(error); }
    finally { fileBusy = false; }
  }

  async function savePause(checkpoint: string) {
    if (!pauseQuest) return;
    pauseSaving = true;
    actionError = '';
    try {
      await journalApi.createLocalSavePoint(pauseQuest.id, checkpoint);
      setUpdated(await journalApi.getQuest(pauseQuest.id));
      pauseQuest = null;
    } catch (error) { actionError = errorMessage(error); }
    finally { pauseSaving = false; }
  }

  async function showContinue(quest: Quest) {
    continueQuest = quest;
    continuePreview = null;
    restoreResult = null;
    actionError = '';
    previewLoading = true;
    try { continuePreview = await journalApi.previewContinue(quest.id); }
    catch (error) { actionError = errorMessage(error); }
    finally { previewLoading = false; }
  }

  async function restore(ids: string[]) {
    if (!continuePreview) return;
    restoring = true;
    actionError = '';
    try { restoreResult = await journalApi.continueQuest(continuePreview.id, ids); }
    catch (error) { actionError = errorMessage(error); }
    finally { restoring = false; await loadQuests(); }
  }

  async function applyState() {
    if (!stateChange) return;
    stateSaving = true;
    actionError = '';
    try {
      const { quest, state } = stateChange;
      setUpdated(await journalApi.updateQuest(quest.id, questInput(quest, state)));
      stateChange = null;
    } catch (error) { actionError = errorMessage(error); }
    finally { stateSaving = false; }
  }

  function setUpdated(updated: Quest) {
    quests = quests.map((quest) => quest.id === updated.id ? updated : quest);
    if (selectedQuest?.id === updated.id) selectedQuest = updated;
  }
  function questInput(quest: Quest, state: QuestState): QuestInput {
    return { icon: quest.icon, title: quest.title, territory: quest.territory, objective: quest.objective, designation: quest.designation, state, current_checkpoint: quest.current_checkpoint };
  }
  function formatDate(value: string | null): string {
    if (!value) return 'Not active yet';
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? 'Date unavailable' : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
  }
  function filename(path: string): string { return path.split(/[\\/]/).at(-1) || path; }
  function closeContinue() { continueQuest = null; continuePreview = null; restoreResult = null; actionError = ''; }
</script>

<svelte:head><title>{selectedQuest ? `${selectedQuest.title} · Quest Journal` : 'Quest Journal'}</title></svelte:head>

<div class="app-shell">
  <header class="window-header" data-tauri-drag-region>
    <span class="window-brand" data-tauri-drag-region><Compass size={15} /> Quest Journal</span>
    <div class="window-controls">
      <button aria-label="Minimize" on:click={() => windowAction('minimize')}><Minus size={14} /></button>
      <button aria-label="Maximize or restore" on:click={() => windowAction('toggleMaximize')}><Square size={12} /></button>
      <button class="close-window" aria-label="Close window" on:click={() => windowAction('close')}><X size={15} /></button>
    </div>
  </header>
  <aside class="sidebar" aria-label="Quests">
    <div class="sidebar-heading"><span>Quests</span><div class="sidebar-heading-actions"><button class="icon-button" title="Pick something" aria-label="Pick something" aria-pressed={shuffleOpen} disabled={loading || materialBusy} bind:this={shuffleTrigger} on:click={showShuffle}><Shuffle size={16} /></button><button class="icon-button" title="New quest" aria-label="New quest" on:click={() => newQuest()}><CirclePlus size={17} /></button></div></div>
    <div class="sidebar-search"><Search size={15} /><input aria-label="Search quests" type="search" bind:value={searchQuery} placeholder="Find a quest…" /></div>
    <nav class="quest-navigation">
      {#each rows as row (row.quest.id)}
        <div class="tree-row" style={`padding-left: ${row.depth * 14}px`}>
          {#if row.children}<button class="tree-toggle" aria-label={`${expanded.has(row.quest.id) ? 'Collapse' : 'Expand'} ${row.quest.title}`} aria-expanded={expanded.has(row.quest.id) || !!normalizedSearch} on:click={() => toggle(row.quest.id)}><ChevronRight size={12} style={`transform: rotate(${expanded.has(row.quest.id) || normalizedSearch ? 90 : 0}deg)`} /></button>{:else}<span class="tree-spacer"></span>{/if}
          <button class="quest-nav-item" class:active={selectedQuest?.id === row.quest.id} class:drop-target={dropTarget === row.quest.id} draggable={!orderBusy && !normalizedSearch} title="Drag to reorder within this level" on:dragstart={(event) => { draggedQuest = row.quest.id; event.dataTransfer?.setData('text/plain', row.quest.id); }} on:dragend={() => { draggedQuest = null; dropTarget = null; }} on:dragover={(event) => { if (validDrop(row.quest)) { event.preventDefault(); dropTarget = row.quest.id; } }} on:dragleave={() => dropTarget = null} on:drop={(event) => dropQuest(row.quest,event)} on:contextmenu={(event) => showQuestMenu(event,row.quest)} on:keydown={(event) => { if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) { event.preventDefault(); showQuestMenu(event as unknown as MouseEvent,row.quest,true); } }} on:click={() => openQuest(row.quest)}><QuestIcon icon={row.quest.icon} size={18} /><span>{row.quest.title}</span></button><span class="quest-order-controls"><button aria-label={`Actions for ${row.quest.title}`} title="Quest actions" aria-haspopup="menu" on:click={(event) => showQuestMenu(event,row.quest,true)}><Ellipsis size={14} /></button><button aria-label={`Move ${row.quest.title} up`} title="Move up" disabled={orderBusy || !canStep(row.quest,-1)} on:click={() => stepQuest(row.quest,-1)}><ArrowUp size={12} /></button><button aria-label={`Move ${row.quest.title} down`} title="Move down" disabled={orderBusy || !canStep(row.quest,1)} on:click={() => stepQuest(row.quest,1)}><ArrowDown size={12} /></button></span>
        </div>
      {/each}
      {#if !loading && !rows.length}<p class="nav-empty">{searchQuery ? 'No matching quests' : 'Your quests will appear here.'}</p>{/if}
    </nav>
    <button class="quest-trash-nav" class:active={questTrashOpen} disabled={questTrashBusy} on:click={() => { questTrashOpen = true; shuffleOpen = false; closeQuestMenu(); }}><Trash2 size={14} /> Quest Trash <span>{deletedQuests.length || ''}</span></button>
  </aside>
  <main>
    {#if pageError}<div class="notice error" role="alert">{pageError}<button class="button ghost" on:click={loadQuests}>Retry</button></div>{/if}
    {#if deletedNotice}<div class="quest-delete-notice" role="status"><span>“{deletedNotice.title}” and its contents moved to Quest Trash.</span><button class="text-action" disabled={questTrashBusy} on:click={() => restoreDeletedQuest(deletedNotice!.id)}><Undo2 size={13} /> Undo</button><button class="icon-button" aria-label="Dismiss deletion notice" on:click={() => deletedNotice = null}><X size={14} /></button></div>{/if}
    {#if questTrashOpen}
      <section class="workspace" aria-labelledby="quest-trash-title"><header class="workspace-toolbar"><span><Trash2 size={14} /> Quest Trash</span><button class="icon-button" title="Back to quest" aria-label="Close Quest Trash" on:click={() => questTrashOpen = false}><X size={15} /></button></header><div class="workspace-body"><header class="quest-heading"><h1 id="quest-trash-title">Deleted quests</h1><p>Restore a quest together with its subquests, tabs, files, and saved sessions. Local files are never deleted.</p></header>
      {#each deletedQuests as quest (quest.id)}<div class="material-row"><QuestIcon icon={quest.icon} size={18} /><div><strong>{quest.title}</strong><small>{quest.parent_id && quests.some(item => item.id === quest.parent_id) ? `Returns to ${ancestors(quests.find(item => item.id === quest.parent_id)!,quests).map(item => item.title).join(' / ')}` : 'Returns to the top level'} · Includes contents deleted with this quest</small></div><button class="button secondary" disabled={questTrashBusy} on:click={() => restoreDeletedQuest(quest.id)}><Undo2 size={14} /> Restore</button></div>{/each}
      {#if !deletedQuests.length}<p class="content-empty">No deleted quests.</p>{/if}</div></section>
    {:else if loading && !selectedQuest}
      <div class="workspace-empty"><RefreshCw class="spin" size={20} /><p>Opening your journal…</p></div>
    {:else if shuffleOpen}
      <ShufflePanel {quests} currentQuest={selectedQuest} onclose={closeShuffle} onopen={openShuffleItem} />
    {:else if selectedQuest}
      <section class="workspace" aria-labelledby="quest-title">
        <header class="workspace-toolbar"><nav class="breadcrumbs" aria-label="Quest breadcrumb">{#each breadcrumbs as crumb, index}{#if index}<ChevronRight size={12} />{/if}<button aria-current={crumb.id === selectedQuest.id ? 'page' : undefined} on:click={() => openQuest(crumb)}>{crumb.title}</button>{/each}</nav><button class="icon-button" aria-label="Edit quest" title="Edit quest" on:click={() => { actionError = ''; formMode = 'edit'; }}><Pencil size={15} /></button></header>
        <div class="workspace-body">
          {#if actionError}<div class="notice error" role="alert">{actionError}</div>{/if}
          <header class="quest-heading"><h1 id="quest-title"><QuestIcon icon={selectedQuest.icon} size={30} /> {selectedQuest.title}</h1>{#if selectedQuest.objective}<p>{selectedQuest.objective}</p>{/if}
            <div class="quest-actions"><button class="button primary" disabled={!selectedQuest.latest_save} on:click={() => showContinue(selectedQuest!)}><Play size={14} /> Continue last save</button><button class="button secondary" disabled={selectedQuest.state === 'completed' || selectedQuest.state === 'abandoned'} on:click={() => { actionError = ''; pauseQuest = selectedQuest; }}><Pause size={14} /> Save & pause</button><span class="quest-status">{selectedQuest.state === 'abandoned' ? 'Set aside' : selectedQuest.state}</span></div>
          </header>
          {#if selectedQuest.current_checkpoint}<div class="checkpoint-line"><span>Where you left off</span><p>{selectedQuest.current_checkpoint}</p></div>{/if}
          <div class="organization-actions"><button class="text-action" on:click={() => newQuest(selectedQuest!.id)}><CirclePlus size={14} /> New quest inside this quest</button><button class="text-action" on:click={() => { destination = selectedQuest?.parent_id ?? ''; moving = !moving; }}>Move quest…</button></div>
          {#if moving}<form class="move-quest" on:submit|preventDefault={moveQuest}><label for="quest-destination">Move “{selectedQuest.title}” to</label><select id="quest-destination" bind:value={destination} disabled={moveBusy}><option value="">Top level</option>{#each destinations as quest}<option value={quest.id}>{ancestors(quest, quests).map(item => item.title).join(' / ')}</option>{/each}</select><button class="button primary" disabled={moveBusy}>{moveBusy ? 'Moving…' : 'Move'}</button><button class="button ghost" type="button" disabled={moveBusy} on:click={() => moving = false}>Cancel</button></form>{/if}
          {#if children.length}<section class="contents-section" aria-label="Nested quests"><div class="contents-heading"><h2>Quests <span>{children.length}</span></h2></div>{#each children as child}<button class="child-quest material-row" on:contextmenu={(event) => showQuestMenu(event,child)} on:click={() => openQuest(child)}><QuestIcon icon={child.icon} size={20} /><div><strong>{child.title}</strong><small>{child.current_checkpoint || child.objective || 'No checkpoint yet'}</small></div><ChevronRight size={14} /></button>{/each}</section>{/if}
          <section class="contents-section" aria-label="Browser tabs">
            <div class="contents-heading"><h2>Browser tabs <span>{browserMaterials.length}</span></h2><button class="text-action" on:click={loadQuests} disabled={materialBusy}><RefreshCw size={13} /> Refresh captures</button></div>
            {#if browserMaterials.length}
              <div class="material-selection"><label><input type="checkbox" aria-label="Select all tabs" checked={selectedMaterials.size === browserMaterials.length} disabled={materialBusy} on:change={(event) => selectedMaterials = event.currentTarget.checked ? new Set(browserMaterials.map(item => item.id)) : new Set()} /> {selectedMaterials.size ? `${selectedMaterials.size} selected` : 'Select tabs'}</label>
              {#if selectedMaterials.size}<button class="text-action" disabled={materialBusy} on:click={() => openMaterials([...selectedMaterials])}><Play size={13} /> Open selected</button><select aria-label="Move selected tabs to quest" bind:value={materialDestination} disabled={materialBusy}><option value="">Move to…</option><option value="__trash__">Trash</option>{#each quests.filter(item => item.id !== selectedQuest?.id) as quest}<option value={quest.id}>{ancestors(quest, quests).map(item => item.title).join(' / ')}</option>{/each}</select><button class="button secondary" disabled={!materialDestination || materialBusy} on:click={moveSelectedMaterials}>{materialDestination === '__trash__' ? 'Move to trash' : 'Move'}</button>{/if}</div>
              <div class="material-list">{#each browserMaterials as resource (resource.id)}<div class="material-row" class:selected={selectedMaterials.has(resource.id)}><input type="checkbox" aria-label={`Select ${materialTitle(resource)}`} checked={selectedMaterials.has(resource.id)} disabled={materialBusy} on:click={(event) => selectMaterial(resource.id, event.shiftKey)} /><button class="material-open" disabled={materialBusy} title="Open tab" on:click={() => openMaterials([resource.id])}><strong>{materialTitle(resource)}</strong><small title={resource.resource_uri}>{resource.resource_uri}</small></button><span class="material-site">{site(resource.resource_uri)}</span><button class="icon-button trash-button" aria-label={`Move ${materialTitle(resource)} to trash`} title="Move to trash" disabled={materialBusy} on:click={() => trashMaterials([resource.id])}><Trash2 size={15} /></button></div>{/each}</div>
            {:else}<p class="content-empty">Capture browser tabs with the Quest Journal extension, or move tabs here from another quest.</p>{/if}
            {#if selectedQuest.trashed_materials.length}<div class="trash-section"><button class="text-action" aria-expanded={showTrash} on:click={() => showTrash = !showTrash}><Trash2 size={13} /> Trash ({selectedQuest.trashed_materials.length}) <ChevronRight size={12} /></button>{#if showTrash}<div class="contents-heading"><span>Trashed tabs</span><button class="text-action" disabled={materialBusy} on:click={() => trashMaterials(selectedQuest!.trashed_materials.map(item => item.id),false)}>Restore all</button></div>{#each selectedQuest.trashed_materials as item (item.id)}<div class="material-row"><Trash2 size={14} /><div><strong>{materialTitle(item)}</strong><small>{item.resource_uri}</small></div><button class="icon-button" title="Restore tab" aria-label={`Restore ${materialTitle(item)}`} disabled={materialBusy} on:click={() => trashMaterials([item.id],false)}><Undo2 size={15} /></button></div>{/each}{/if}</div>{/if}
            {#if materialNotice}<p class="material-notice" role="status">{materialNotice}</p>{/if}
          </section>
          <section class="contents-section" aria-label="Files">
            <div class="contents-heading"><h2>Files <span>{selectedQuest.files.length}</span></h2><button class="text-action" on:click={attachFiles} disabled={fileBusy}><FilePlus2 size={14} /> {fileBusy ? 'Attaching…' : 'Attach files'}</button></div>
            {#each selectedQuest.files as file (file.id)}<div class="material-row"><FileText size={16} /><div><strong>{file.label || filename(file.path)}</strong><small>{file.path}</small></div><button class="icon-button" aria-label={`Remove reference to ${file.label || filename(file.path)}`} disabled={fileBusy} on:click={() => removeFile(file.id)}><Trash2 size={14} /></button></div>{/each}
            {#if !selectedQuest.files.length}<p class="content-empty">Attach notes, documents, or working files.</p>{/if}
          </section>
          <footer class="workspace-footer"><span>{selectedQuest.latest_save ? `Last saved ${formatDate(selectedQuest.latest_save.created_at)}` : 'No saved session yet'}</span><div><button class="text-action" disabled={selectedQuest.state === 'completed'} on:click={() => stateChange = { quest: selectedQuest!, state: 'completed' }}><Check size={14} /> Complete</button><button class="text-action" disabled={selectedQuest.state === 'abandoned'} on:click={() => stateChange = { quest: selectedQuest!, state: 'abandoned' }}><Archive size={14} /> Set aside</button></div></footer>
        </div>
      </section>
    {:else}
      <div class="workspace-empty"><Compass size={28} /><h1>A place to return to</h1><p>Create a quest for something you want to explore or work on.</p><button class="button primary" on:click={() => newQuest()}>Create a quest</button></div>
    {/if}
  </main>
</div>

{#if formMode}<Dialog open title={formMode === 'create' ? 'Create a quest' : 'Edit quest'} description={formMode === 'create' && createParentId ? `Inside ${quests.find(quest => quest.id === createParentId)?.title}` : 'Keep it broad enough to return to, and specific enough to recognize.'} onclose={() => { if (!formSaving) formMode = null; }}><QuestForm quest={formMode === 'edit' ? selectedQuest : null} saving={formSaving} error={actionError} onsubmit={saveQuest} oncancel={() => formMode = null} /></Dialog>{/if}
{#if pauseQuest}<Dialog open title={`Pause ${pauseQuest.title}`} description="Create a local save point for the next time you return." onclose={() => { if (!pauseSaving) pauseQuest = null; }}><PauseDialog quest={pauseQuest} saving={pauseSaving} error={actionError} onsave={savePause} oncancel={() => pauseQuest = null} /></Dialog>{/if}
{#if continueQuest}<Dialog open wide title={`Continue ${continueQuest.title}`} description="Choose exactly which saved resources to reopen." onclose={closeContinue}>{#if previewLoading}<div class="modal-loading" aria-live="polite"><RefreshCw class="spin" size={20} /> Loading saved context…</div>{:else if continuePreview}<ContinueDialog savePoint={continuePreview} restoring={restoring} error={actionError} result={restoreResult} onrestore={restore} onclose={closeContinue} />{:else}<div class="notice error" role="alert">{actionError || 'No saved context is available for this quest.'}</div><footer class="dialog-actions"><button class="button ghost" type="button" on:click={closeContinue}>Close</button><button class="button secondary" type="button" on:click={() => showContinue(continueQuest!)}><RefreshCw size={16} /> Try again</button></footer>{/if}</Dialog>{/if}
{#if stateChange}<Dialog open title={stateChange.state === 'completed' ? 'Mark this quest completed?' : 'Set this quest aside?'} description={stateChange.state === 'completed' ? 'It will remain in your journal and can be changed later.' : 'There is no penalty or lost history. You can return whenever you choose.'} onclose={() => { if (!stateSaving) stateChange = null; }}>{#if actionError}<div class="notice error" role="alert">{actionError}</div>{/if}<p class="confirm-copy">{stateChange.quest.title}</p><footer class="dialog-actions"><button class="button ghost" type="button" on:click={() => stateChange = null} disabled={stateSaving}>Cancel</button><button class:danger={stateChange.state === 'abandoned'} class:primary={stateChange.state === 'completed'} class="button" type="button" on:click={applyState} disabled={stateSaving}>{stateSaving ? 'Saving…' : stateChange.state === 'completed' ? 'Mark completed' : 'Set aside'}</button></footer></Dialog>{/if}

{#if questMenu}<QuestMenu x={questMenu.x} y={questMenu.y} title={questMenu.quest.title} hasChildren={quests.some(item => item.parent_id === questMenu?.quest.id)} onaction={questMenuAction} onclose={closeQuestMenu} />{/if}
{#if renameTarget}<Dialog open title="Rename quest" onclose={() => { if (!renameBusy) renameTarget = null; }}><form on:submit|preventDefault={renameQuest}><div class="field"><label for="rename-quest-title">Name</label><input id="rename-quest-title" bind:value={renameTitle} maxlength="120" disabled={renameBusy} /></div>{#if renameError}<p class="notice error" role="alert">{renameError}</p>{/if}<footer class="dialog-actions"><button type="button" class="button ghost" disabled={renameBusy} on:click={() => renameTarget = null}>Cancel</button><button class="button primary" disabled={renameBusy || renameTitle.trim().length < 2}>{renameBusy ? 'Renaming…' : 'Rename'}</button></footer></form></Dialog>{/if}
