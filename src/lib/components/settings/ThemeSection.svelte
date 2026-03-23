<script lang="ts">
  import { settingsStore, savePartialSettings } from '$lib/stores/settings.svelte';
  import { invoke } from '$lib/api/invoke';
  import type { ThemeFile, ThemeListEntry } from '$lib/types';
  import { applyTheme, saveCustomTheme, deleteCustomTheme, generateThemeTemplate } from '$lib/stores/theme.svelte';
  import { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from '$lib/constants';
  import { i18n } from '$lib/i18n/index.svelte';
  import { setLocale, AVAILABLE_LOCALES, getLocale } from '$lib/i18n/index.svelte';
  import type { Locale } from '$lib/i18n/types';

  const t = $derived(i18n.t);

  let {
    saving = $bindable(false),
    themes,
    selectedThemeId,
  }: {
    saving: boolean;
    themes: ThemeListEntry[];
    selectedThemeId: string;
  } = $props();

  let showThemeEditor = $state(false);
  let themeJsonText = $state('');
  let themeError = $state('');
  let editingThemeId = $state<string | null>(null);

  function isThemeFile(value: unknown): value is ThemeFile {
    return typeof value === 'object' && value !== null && 'name' in value && 'colors' in value;
  }

  function isCustomTheme(id: string): boolean {
    return id !== DEFAULT_DARK_THEME_ID && id !== DEFAULT_LIGHT_THEME_ID;
  }

  async function handleThemeChange(e: Event) {
    const target = e.target;
    if (!(target instanceof HTMLSelectElement)) return;
    const themeId = target.value;
    saving = true;
    try {
      await applyTheme(themeId);
      await savePartialSettings({ theme: themeId });
    } catch (e) {
      themeError = String(e);
    } finally {
      saving = false;
    }
  }

  function openNewThemeEditor() {
    editingThemeId = null;
    themeJsonText = generateThemeTemplate('dark');
    themeError = '';
    showThemeEditor = true;
  }

  async function openEditThemeEditor(themeId: string) {
    try {
      const theme = await invoke<ThemeFile>('get_theme', { themeId });
      editingThemeId = themeId;
      themeJsonText = JSON.stringify(theme, null, 2);
      themeError = '';
      showThemeEditor = true;
    } catch {
      themeError = t.settings.theme.loadFailed;
    }
  }

  async function handleSaveTheme() {
    themeError = '';
    let parsed: unknown;
    try {
      parsed = JSON.parse(themeJsonText);
    } catch {
      themeError = t.settings.theme.invalidJson;
      return;
    }
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
      themeError = t.settings.theme.invalidJson;
      return;
    }
    if (!isThemeFile(parsed)) {
      themeError = 'Invalid theme file format';
      return;
    }
    const themeData = parsed;
    if (!themeData.name || !themeData.type) {
      themeError = t.settings.theme.nameTypeRequired;
      return;
    }
    if (themeData.type !== 'dark' && themeData.type !== 'light') {
      themeError = t.settings.theme.invalidType;
      return;
    }
    const themeId = editingThemeId ?? themeData.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    if (!themeId || themeId === DEFAULT_DARK_THEME_ID || themeId === DEFAULT_LIGHT_THEME_ID) {
      themeError = t.settings.theme.invalidName;
      return;
    }
    saving = true;
    try {
      await saveCustomTheme(themeId, themeData);
      await applyTheme(themeId);
      await savePartialSettings({ theme: themeId });
      showThemeEditor = false;
    } catch (e) {
      themeError = t.settings.theme.saveFailed(String(e));
    } finally {
      saving = false;
    }
  }

  async function handleDeleteTheme(themeId: string) {
    saving = true;
    try {
      await deleteCustomTheme(themeId);
      if (settingsStore.settings?.theme === themeId) {
        await savePartialSettings({ theme: DEFAULT_DARK_THEME_ID });
      }
    } catch (e) {
      themeError = t.settings.theme.deleteFailed(String(e));
    } finally {
      saving = false;
    }
  }

  async function handleLanguageChange(e: Event) {
    const target = e.target;
    if (!(target instanceof HTMLSelectElement)) return;
    const raw = target.value;
    const matched = AVAILABLE_LOCALES.find(l => l.code === raw);
    if (!matched) return;
    const locale: Locale = matched.code;
    setLocale(locale);
    saving = true;
    try {
      await savePartialSettings({ language: locale });
    } catch {
      // savePartialSettings already sets lastError on settingsStore;
      // surface it to the user via the themeError display
      themeError = settingsStore.lastError ?? t.settings.theme.saveFailed('unknown');
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings-section">
  <h4>{t.settings.theme.title}</h4>

  <div class="setting-item">
    <label class="interval-label">
      <span>{t.settings.theme.select}</span>
      <select
        class="theme-select"
        value={selectedThemeId}
        onchange={handleThemeChange}
        disabled={saving}
      >
        {#each themes as theme (theme.id)}
          <option value={theme.id}>
            {theme.name} ({theme.type === 'dark' ? t.settings.theme.dark : t.settings.theme.light})
          </option>
        {/each}
      </select>
    </label>
  </div>

  <div class="setting-item theme-actions">
    <button class="debug-btn" onclick={openNewThemeEditor}>
      {t.settings.theme.createCustom}
    </button>
    {#if isCustomTheme(selectedThemeId)}
      <button class="debug-btn" onclick={() => openEditThemeEditor(selectedThemeId)}>
        {t.settings.theme.edit}
      </button>
      <button class="debug-btn danger" onclick={() => handleDeleteTheme(selectedThemeId)}>
        {t.settings.theme.delete}
      </button>
    {/if}
  </div>

  {#if showThemeEditor}
    <div class="theme-editor">
      <p class="setting-desc setting-desc-spaced">
        {t.settings.theme.editorDesc}
      </p>
      <textarea
        class="theme-json-input"
        bind:value={themeJsonText}
        rows={16}
        spellcheck={false}
      ></textarea>
      {#if themeError}
        <p class="error-message">{themeError}</p>
      {/if}
      <div class="theme-editor-actions">
        <button class="debug-btn" onclick={handleSaveTheme} disabled={saving}>
          {saving ? t.settings.theme.saving : t.settings.theme.saveAndApply}
        </button>
        <button class="debug-btn" onclick={() => showThemeEditor = false}>
          {t.settings.theme.cancel}
        </button>
      </div>
    </div>
  {/if}
</div>

<div class="settings-section">
  <h4>{t.settings.language.title}</h4>

  <div class="setting-item">
    <label class="interval-label">
      <span>{t.settings.language.title}</span>
      <select
        class="theme-select"
        value={getLocale()}
        onchange={handleLanguageChange}
        disabled={saving}
      >
        {#each AVAILABLE_LOCALES as locale (locale.code)}
          <option value={locale.code}>{locale.name}</option>
        {/each}
      </select>
    </label>
  </div>
</div>

<style>
  .settings-section h4 {
    font-size: var(--termi-font-size-sm, 14px);
    font-weight: 600;
    color: var(--termi-text-secondary);
    margin-bottom: var(--termi-spacing-md-sm, 12px);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .setting-item {
    margin-bottom: var(--termi-spacing-md, 16px);
  }

  .setting-desc {
    font-size: var(--termi-font-size-xs, 12px);
    color: var(--termi-text-secondary);
    margin: var(--termi-spacing-xs, 4px) 0 0 0;
  }

  .setting-desc-spaced {
    margin-bottom: var(--termi-spacing-sm, 8px);
  }

  .interval-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--termi-spacing-md-sm, 12px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 14px);
  }

  .error-message {
    margin-top: var(--termi-spacing-md-sm, 12px);
  }

  .theme-select {
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-sm, 8px);
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    font-size: var(--termi-font-size-sm, 14px);
    font-family: inherit;
    cursor: pointer;
    min-width: 160px;
  }

  .theme-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .theme-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .theme-editor {
    margin-top: var(--termi-spacing-sm, 8px);
  }

  .theme-json-input {
    width: 100%;
    background-color: var(--termi-bg-secondary);
    color: var(--termi-text-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    padding: var(--termi-spacing-sm, 8px);
    font-family: var(--termi-font-mono);
    font-size: var(--termi-font-size-xs, 12px);
    line-height: 1.5;
    resize: vertical;
    tab-size: 2;
  }

  .theme-json-input:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -1px;
    border-color: var(--termi-accent);
  }

  .theme-editor-actions {
    display: flex;
    gap: 6px;
    margin-top: var(--termi-spacing-sm, 8px);
  }

  .debug-btn {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-secondary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    padding: 3px 10px;
    font-size: var(--termi-font-size-xs, 12px);
    cursor: pointer;
    white-space: nowrap;
  }

  .debug-btn:hover {
    color: var(--termi-text-primary);
    border-color: var(--termi-accent);
  }

  .debug-btn:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .debug-btn.danger:hover {
    border-color: var(--termi-danger);
    color: var(--termi-danger);
  }

  .theme-select:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }
</style>
