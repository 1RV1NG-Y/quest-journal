import type { Quest } from './types';

export interface ShuffleFilters {
  topLevel: boolean;
  subquests: boolean;
  elements: boolean;
}

export interface ShuffleItem {
  key: string;
  kind: 'quest' | 'tab' | 'file';
  id: string;
  questId: string;
  title: string;
  context: string;
  location?: string;
  icon?: string;
}

export function buildShufflePool(quests: Quest[], rootId: string | null, filters: ShuffleFilters): ShuffleItem[] {
  const byId = new Map(quests.map(quest => [quest.id, quest]));
  const pool: ShuffleItem[] = [];
  for (const quest of quests) {
    const lineage: Quest[] = [];
    const seen = new Set<string>();
    let ancestor: Quest | undefined = quest;
    while (ancestor && !seen.has(ancestor.id)) {
      lineage.unshift(ancestor);
      seen.add(ancestor.id);
      ancestor = ancestor.parent_id ? byId.get(ancestor.parent_id) : undefined;
    }
    if (rootId !== null && !seen.has(rootId)) continue;
    const path = lineage.map(item => item.title).join(' → ');
    if (quest.parent_id ? filters.subquests : filters.topLevel) {
      pool.push({ key: `quest:${quest.id}`, kind: 'quest', id: quest.id, questId: quest.id,
        title: quest.title, icon: quest.icon, context: `${lineage.slice(0, -1).map(item => item.title).join(' → ')}${quest.parent_id ? ' · Subquest' : 'Top-level quest'}` });
    }
    if (!filters.elements) continue;
    for (const tab of quest.materials) {
      const title = (tab.state_json as { title?: unknown } | null)?.title;
      pool.push({ key: `tab:${tab.id}`, kind: 'tab', id: tab.id, questId: quest.id,
        title: typeof title === 'string' && title.trim() ? title : tab.resource_uri,
        context: `${path} · Browser tab`, location: tab.resource_uri });
    }
    for (const file of quest.files) {
      pool.push({ key: `file:${file.id}`, kind: 'file', id: file.id, questId: quest.id,
        title: file.label || file.path.split(/[\\/]/).at(-1) || file.path,
        context: `${path} · File`, location: file.path });
    }
  }
  return pool;
}

// Every eligible item has equal odds, independently of the decorative animation.
export function pickShuffleItem(pool: ShuffleItem[], random = Math.random): ShuffleItem | null {
  return pool.length ? pool[Math.floor(random() * pool.length)] : null;
}
