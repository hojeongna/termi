<script lang="ts">
  import { onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { invoke } from '$lib/api/invoke';
  import Icon from '$lib/components/Icon.svelte';
  import { themeStore, loadAvailableThemes } from '$lib/stores/theme.svelte';
  import { i18n } from '$lib/i18n/index.svelte';
  import ThemeSection from '$lib/components/settings/ThemeSection.svelte';
  import ReminderSection from '$lib/components/settings/ReminderSection.svelte';
  import HookSection from '$lib/components/settings/HookSection.svelte';
  import DebugLogSection from '$lib/components/settings/DebugLogSection.svelte';
  import { CLOSE_ICON_SIZE } from '$lib/constants';

  const t = $derived(i18n.t);

  let { onClose }: { onClose: () => void } = $props();

  let saving = $state(false);

  // 테마 상태
  let themes = $derived(themeStore.availableThemes);
  let selectedThemeId = $derived(themeStore.currentThemeId);

  // Hook 등록 상태
  let hookRegistered = $state(false);
  let hookPath = $state<string | null>(null);
  let hookLoading = $state(false);
  let hookError = $state('');

  onMount(() => {
    loadAvailableThemes();
    loadHookStatus();
  });

  async function loadHookStatus() {
    try {
      const status = await invoke<{ registered: boolean; hookPath: string | null }>('get_hook_status');
      hookRegistered = status.registered;
      hookPath = status.hookPath;
    } catch (e) {
      hookError = String(e);
    }
  }
</script>

<div class="settings-panel">
  <div class="settings-header">
    <h3>{t.settings.title}</h3>
    <button class="close-btn" onclick={onClose}><Icon name="close" size={CLOSE_ICON_SIZE} /></button>
  </div>

  <ThemeSection bind:saving {themes} {selectedThemeId} />

  <ReminderSection bind:saving />

  <HookSection bind:hookRegistered bind:hookPath bind:hookLoading bind:hookError />

  <DebugLogSection />

  {#if settingsStore.lastError}
    <p class="error-message">{settingsStore.lastError}</p>
  {/if}
</div>

<style>
  .settings-panel {
    padding: var(--termi-spacing-md, 16px);
    overflow-y: auto;
    height: 100%;
  }

  .settings-header {
    position: sticky;
    top: 0;
    z-index: 1;
    background-color: var(--termi-bg-primary);
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--termi-spacing-lg, 20px);
    padding-bottom: var(--termi-spacing-sm, 8px);
  }

  .settings-header h3 {
    font-size: var(--termi-font-size-lg, 18px);
    font-weight: 600;
    color: var(--termi-text-primary);
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-xl, 20px);
    cursor: pointer;
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-sm, 8px);
    border-radius: var(--termi-radius-sm, 4px);
  }

  .close-btn:hover {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
  }

  .error-message {
    margin-top: var(--termi-spacing-md-sm, 12px);
  }

  /* Apply section separators across child components */
  .settings-panel :global(.settings-section + .settings-section) {
    margin-top: var(--termi-spacing-lg, 20px);
    padding-top: var(--termi-spacing-lg, 20px);
    border-top: 1px solid var(--termi-border);
  }
</style>
