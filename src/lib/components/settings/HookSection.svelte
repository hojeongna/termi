<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { invoke } from '$lib/api/invoke';
  import { i18n } from '$lib/i18n/index.svelte';

  const t = $derived(i18n.t);

  let {
    hookRegistered = $bindable(false),
    hookPath = $bindable<string | null>(null),
    hookLoading = $bindable(false),
    hookError = $bindable(''),
  }: {
    hookRegistered: boolean;
    hookPath: string | null;
    hookLoading: boolean;
    hookError: string;
  } = $props();

  async function handleRegisterHooks() {
    hookLoading = true;
    hookError = '';
    try {
      const status = await invoke<{ registered: boolean; hookPath: string | null }>('register_hooks');
      hookRegistered = status.registered;
      hookPath = status.hookPath;
    } catch (e) {
      hookError = t.settings.hooks.registerFailed(String(e));
    } finally {
      hookLoading = false;
    }
  }

  async function handleUnregisterHooks() {
    const confirmed = await confirm(t.settings.hooks.unregisterConfirm, { kind: 'warning' });
    if (!confirmed) return;
    hookLoading = true;
    hookError = '';
    try {
      const status = await invoke<{ registered: boolean; hookPath: string | null }>('unregister_hooks');
      hookRegistered = status.registered;
      hookPath = status.hookPath;
    } catch (e) {
      hookError = t.settings.hooks.unregisterFailed(String(e));
    } finally {
      hookLoading = false;
    }
  }
</script>

<div class="settings-section">
  <h4>{t.settings.hooks.title}</h4>

  <div class="setting-item">
    <div class="hook-status-row">
      <span class="hook-status-badge" class:registered={hookRegistered}>
        {hookRegistered ? t.settings.hooks.registered : t.settings.hooks.notRegistered}
      </span>
      {#if hookRegistered}
        <button
          class="debug-btn danger"
          onclick={handleUnregisterHooks}
          disabled={hookLoading}
        >
          {hookLoading ? t.settings.hooks.unregistering : t.settings.hooks.unregister}
        </button>
      {:else}
        <button
          class="debug-btn"
          onclick={handleRegisterHooks}
          disabled={hookLoading}
        >
          {hookLoading ? t.settings.hooks.registering : t.settings.hooks.register}
        </button>
      {/if}
    </div>
    <p class="setting-desc">{t.settings.hooks.desc}</p>
    {#if hookError}
      <p class="error-message">{hookError}</p>
    {/if}
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

  .error-message {
    margin-top: var(--termi-spacing-md-sm, 12px);
  }

  .hook-status-row {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-sm, 8px);
  }

  .hook-status-badge {
    font-size: var(--termi-font-size-xs, 12px);
    padding: 2px 8px;
    border-radius: var(--termi-radius-sm, 4px);
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-secondary);
    border: 1px solid var(--termi-border);
  }

  .hook-status-badge.registered {
    background-color: var(--termi-accent);
    color: var(--termi-text-primary);
    border-color: var(--termi-accent);
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
</style>
