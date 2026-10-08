import { defineConfig } from 'wxt';
import { EXTENSION_VERSION } from './utils/version';

const CHROME_PUBLIC_KEY =
  'MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAizmpoxOTOIzf2ySb6Qqr8S5OSK+SBxTJgnOMUjIRd2/Jpg+N4+AJpbQb7hREWwpSuD5+I1Q85+Dx9e9ZbJ6LYqhE7b91LOmBZGbwbJb72uqLlIqg6l6vmhTPfkKII+VKwQoXQnSkid9gQrn/KRhpEcYo26XezCmsQqtMJppwJ0BsjsGI0yNVP6bgTuXOzRgi66Jum3rdfTGd9zwwrqZVeFLw/FW43/8yO/FuZ8xGREW8/LV4Hok1sMHwJ90BVdrxsj8JnIeSuJRCNaw6JYApNLR1J+RZ4FwiybhXkIlsbJWIRIWc9tVQJ6H9Z9xPStbHXNkwNL3msse8It23hCpe1wIDAQAB';

export default defineConfig({
  modules: ['@wxt-dev/module-svelte'],
  manifest: {
    name: 'Quest Journal',
    description: 'Organize browser tabs into quests and save sessions across windows.',
    version: EXTENSION_VERSION,
    key: CHROME_PUBLIC_KEY,
    permissions: ['nativeMessaging', 'tabs', 'storage'],
    action: {
      default_title: 'Add tabs to Quest Journal',
    },
    browser_specific_settings: {
      gecko: {
        id: 'quest-journal@local',
        strict_min_version: '109.0',
      },
    },
  },
});
