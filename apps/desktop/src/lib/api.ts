import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { ContinueResult, Quest, QuestInput, SavePoint, RestoreOutcome } from './types';

export const journalApi = {
  listQuests: () => invoke<Quest[]>('list_quests'),
  trashQuest: (id: string) => invoke<void>('trash_quest', { id }),
  listTrashedQuests: () => invoke<Quest[]>('list_trashed_quests'),
  restoreQuest: (id: string) => invoke<void>('restore_quest', { id }),
  getQuest: (id: string) => invoke<Quest>('get_quest', { id }),
  createQuest: (input: QuestInput) => invoke<Quest>('create_quest', { input }),
  updateQuest: (id: string, input: QuestInput) => invoke<Quest>('update_quest', { id, input }),
  reorderQuests: (parentId: string | null, ids: string[]) => invoke<void>('reorder_quests', { parentId, ids }),
  moveQuest: (id: string, parentId: string | null) => invoke<Quest>('move_quest', { id, parentId }),
  setMaterialsTrashed: (questId: string, ids: string[], trashed: boolean) => invoke<void>('set_materials_trashed', { questId, ids, trashed }),
  moveMaterials: (source: string, destination: string, ids: string[]) => invoke<void>('move_materials', { source, destination, ids }),
  openMaterials: (questId: string, ids: string[]) => invoke<ContinueResult>('open_materials', { questId, ids }),
  openQuestFile: (questId: string, id: string) => invoke<RestoreOutcome>('open_quest_file', { questId, id }),
  addQuestFiles: (questId: string, paths: string[]) =>
    invoke<Quest>('add_quest_files', { questId, paths }),
  removeQuestFile: (id: string) => invoke<void>('remove_quest_file', { id }),
  createLocalSavePoint: (questId: string, checkpoint: string) =>
    invoke<SavePoint>('create_local_save_point', { questId, checkpoint }),
  previewContinue: (questId: string) => invoke<SavePoint>('preview_continue', { questId }),
  continueQuest: (savePointId: string, resourceIds: string[] | null) =>
    invoke<ContinueResult>('continue_quest', { savePointId, resourceIds }),
  chooseFiles: async (): Promise<string[]> => {
    const selected = await open({ multiple: true, directory: false, title: 'Attach files to quest' });
    if (!selected) return [];
    return Array.isArray(selected) ? selected : [selected];
  }
};

export function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  return 'Something went wrong. Try again.';
}
