<script lang="ts">
  import { settingsStore, savePartialSettings } from '$lib/stores/settings.svelte';
  import { i18n } from '$lib/i18n/index.svelte';
  import { DEFAULT_INTERVAL_MINUTES, MAX_INTERVAL_MINUTES, DEFAULT_MAX_REPEAT, MAX_REPEAT_LIMIT } from '$lib/constants';

  const t = $derived(i18n.t);

  let {
    saving = $bindable(false),
  }: {
    saving: boolean;
  } = $props();

  let reminderEnabled = $derived(settingsStore.settings?.reminder.enabled ?? true);
  let intervalMinutes = $derived(settingsStore.settings?.reminder.intervalMinutes ?? DEFAULT_INTERVAL_MINUTES);
  let maxRepeat = $derived(settingsStore.settings?.reminder.maxRepeat ?? DEFAULT_MAX_REPEAT);

  function getInputValue(e: Event): number | null {
    const target = e.target;
    if (target instanceof HTMLInputElement) {
      return parseInt(target.value);
    }
    return null;
  }

  async function handleToggle() {
    saving = true;
    try {
      await savePartialSettings({
        reminder: { enabled: !reminderEnabled, intervalMinutes, maxRepeat },
      });
    } catch {
      // savePartialSettings already sets lastError
    } finally {
      saving = false;
    }
  }

  async function handleIntervalChange(e: Event) {
    const value = getInputValue(e);
    if (value !== null && value > 0 && value <= MAX_INTERVAL_MINUTES) {
      saving = true;
      try {
        await savePartialSettings({
          reminder: { enabled: reminderEnabled, intervalMinutes: value, maxRepeat },
        });
      } catch {
        // savePartialSettings already sets lastError
      } finally {
        saving = false;
      }
    }
  }

  async function handleMaxRepeatChange(e: Event) {
    const value = getInputValue(e);
    if (value !== null && value >= 0 && value <= MAX_REPEAT_LIMIT) {
      saving = true;
      try {
        await savePartialSettings({
          reminder: { enabled: reminderEnabled, intervalMinutes, maxRepeat: value },
        });
      } catch {
        // savePartialSettings already sets lastError
      } finally {
        saving = false;
      }
    }
  }
</script>

<div class="settings-section">
  <h4>{t.settings.alwaysOnTop.title}</h4>
  <div class="setting-item">
    <label class="toggle-label">
      <input
        type="checkbox"
        checked={settingsStore.settings?.alwaysOnTop ?? false}
        onchange={async (e) => {
          const target = e.target;
          if (!(target instanceof HTMLInputElement)) return;
          try {
            await savePartialSettings({ alwaysOnTop: target.checked });
          } catch {
            // savePartialSettings already sets lastError
          }
        }}
      />
      <span>{t.settings.alwaysOnTop.enabled}</span>
    </label>
    <p class="setting-desc">{t.settings.alwaysOnTop.enabledDesc}</p>
  </div>
</div>

<div class="settings-section">
  <h4>{t.settings.reminder.title}</h4>

  <div class="setting-item">
    <label class="toggle-label">
      <input
        type="checkbox"
        checked={reminderEnabled}
        onchange={handleToggle}
        disabled={saving}
      />
      <span>{t.settings.reminder.enabled}</span>
    </label>
    <p class="setting-desc">{t.settings.reminder.enabledDesc}</p>
  </div>

  <div class="setting-item" class:disabled={!reminderEnabled}>
    <label class="interval-label">
      <span>{t.settings.reminder.interval}</span>
      <input
        type="number"
        min="1"
        max={MAX_INTERVAL_MINUTES}
        value={intervalMinutes}
        onchange={handleIntervalChange}
        disabled={!reminderEnabled || saving}
      />
    </label>
  </div>

  <div class="setting-item" class:disabled={!reminderEnabled}>
    <label class="interval-label">
      <span>{t.settings.reminder.maxRepeat}</span>
      <input
        type="number"
        min="0"
        max={MAX_REPEAT_LIMIT}
        value={maxRepeat}
        onchange={handleMaxRepeatChange}
        disabled={!reminderEnabled || saving}
      />
    </label>
    <p class="setting-desc">{t.settings.reminder.maxRepeatDesc}</p>
  </div>
</div>

<div class="settings-section">
  <h4>{t.settings.autoAttach.title}</h4>
  <div class="setting-item">
    <label class="toggle-label">
      <input
        type="checkbox"
        checked={settingsStore.settings?.autoAttachEnabled ?? true}
        onchange={async (e) => {
          const target = e.target;
          if (!(target instanceof HTMLInputElement)) return;
          try {
            await savePartialSettings({ autoAttachEnabled: target.checked });
          } catch {
            // savePartialSettings already sets lastError
          }
        }}
      />
      <span>{t.settings.autoAttach.enabled}</span>
    </label>
    <p class="setting-desc">{t.settings.autoAttach.enabledDesc}</p>
  </div>
</div>

{#if settingsStore.lastError}
  <p class="error-message">{settingsStore.lastError}</p>
{/if}

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

  .setting-item.disabled {
    opacity: 0.5;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-sm, 8px);
    cursor: pointer;
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 14px);
  }

  .toggle-label input[type="checkbox"] {
    width: var(--termi-spacing-md, 16px);
    height: var(--termi-spacing-md, 16px);
    accent-color: var(--termi-accent);
    cursor: pointer;
  }

  .setting-desc {
    font-size: var(--termi-font-size-xs, 12px);
    color: var(--termi-text-secondary);
    margin: var(--termi-spacing-xs, 4px) 0 0 0;
  }

  .interval-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--termi-spacing-md-sm, 12px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 14px);
  }

  .interval-label input[type="number"] {
    width: var(--termi-input-width-sm, 64px);
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-sm, 8px);
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    font-size: var(--termi-font-size-sm, 14px);
    text-align: center;
  }

  .interval-label input[type="number"]:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle-label input[type="checkbox"]:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .interval-label input[type="number"]:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }
</style>
