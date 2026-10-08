<script lang="ts">
  import { FolderOpen } from '@lucide/svelte';
  import { generalIcons, techIcons } from './quest-icons';
  export let icon: string | undefined = '';
  export let size = 16;
  $: tech = techIcons.find(item => item.id === icon);
  $: GeneralIcon = generalIcons.find(item => item.id === icon)?.component ?? FolderOpen;
</script>
<span class="quest-icon" aria-hidden="true" style={`width:${size}px;height:${size}px`}>
  {#if tech}<img src={tech.src} alt="" width={size} height={size} class:light={tech.light} draggable="false" />{:else}<svelte:component this={GeneralIcon} {size} />{/if}
</span>
<style>
  .quest-icon{display:inline-flex;align-items:center;justify-content:center;flex:0 0 auto;vertical-align:middle;color:inherit}
  img{object-fit:contain;display:block}.light{background:#e2e2e2;border-radius:3px;padding:1px}
</style>
