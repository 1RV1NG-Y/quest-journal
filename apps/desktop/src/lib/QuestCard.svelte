<script lang="ts">
  import { ArrowRight, Compass, Pause, Play } from '@lucide/svelte';
  import type { Quest } from './types';

  export let quest: Quest;
  export let prominent = false;
  export let onopen: () => void;
  export let onpause: () => void;
  export let oncontinue: () => void;

  const stateLabel: Record<Quest['state'], string> = {
    active: 'Active', paused: 'Paused', dormant: 'Dormant', waiting: 'Waiting', completed: 'Completed', abandoned: 'Set aside'
  };
</script>

<article class:prominent class="quest-card">
  <button class="card-open" type="button" on:click={onopen} aria-label={`Open ${quest.title}`}>
    <span class="eyebrow"><Compass size={14} /> {quest.territory}</span>
    <span class="title-row"><strong>{quest.title}</strong><ArrowRight size={18} /></span>
    <span class="objective">{quest.objective}</span>
    <span class="checkpoint"><span>Next</span>{quest.current_checkpoint || 'Add a checkpoint when you know where to begin.'}</span>
  </button>
  <footer>
    <span class:complete={quest.state === 'completed'} class="state"><i></i>{stateLabel[quest.state]}</span>
    <div class="card-actions">
      <button class="button compact ghost" type="button" on:click={onpause} disabled={quest.state === 'completed' || quest.state === 'abandoned'}><Pause size={15} /> Pause</button>
      <button class="button compact secondary" type="button" on:click={oncontinue} disabled={!quest.latest_save}><Play size={15} /> Continue</button>
    </div>
  </footer>
</article>

<style>
  .quest-card { display: flex; min-width: 0; flex-direction: column; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-lg); transition: border-color var(--transition), transform var(--transition); }
  .quest-card:hover { border-color: var(--border-strong); transform: translateY(-1px); }
  .quest-card.prominent { background: linear-gradient(120deg, var(--surface-raised), var(--surface)); border-color: var(--accent-border); }
  .card-open { display: grid; flex: 1; gap: var(--space-3); width: 100%; padding: var(--space-5); color: inherit; text-align: left; background: none; border: 0; border-radius: var(--radius-lg) var(--radius-lg) 0 0; cursor: pointer; }
  .eyebrow { display: flex; align-items: center; gap: var(--space-2); color: var(--accent); font-size: var(--text-xs); font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .title-row { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-4); }
  .title-row strong { font-family: var(--font-display); font-size: var(--text-xl); font-weight: 650; letter-spacing: -.02em; line-height: 1.2; }
  .title-row :global(svg) { flex: 0 0 auto; margin-top: .2rem; color: var(--text-faint); }
  .objective { display: -webkit-box; overflow: hidden; color: var(--text-soft); font-size: var(--text-sm); line-height: 1.55; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; }
  .checkpoint { display: grid; gap: var(--space-1); padding-top: var(--space-3); color: var(--text-soft); font-size: var(--text-sm); border-top: 1px solid var(--border); }
  .checkpoint > span { color: var(--text-faint); font-size: var(--text-xs); font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  footer { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) var(--space-4); border-top: 1px solid var(--border); }
  .state { display: flex; align-items: center; gap: var(--space-2); color: var(--text-muted); font-size: var(--text-xs); font-weight: 600; }
  .state i { width: .42rem; height: .42rem; background: var(--accent-blue); border-radius: 50%; }
  .state.complete i { background: var(--success); }
  .card-actions { display: flex; gap: var(--space-2); }
  @media (max-width: 440px) { footer { align-items: flex-start; flex-direction: column; } .card-actions { width: 100%; } .card-actions .button { flex: 1; } }
</style>
