<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import type { ExternalTerminalInfo } from '$lib/types';
  import { i18n } from '$lib/i18n/index.svelte';
  import { MODAL_CLOSE_ICON_SIZE, MODAL_ITEM_ICON_SIZE, MODAL_ACTION_ICON_SIZE } from '$lib/constants';

  const t = $derived(i18n.t);

  let {
    externalTerminals,
    onclose,
    onattach,
    importing = false,
  }: {
    externalTerminals: ExternalTerminalInfo[];
    onclose: () => void;
    onattach: (hwnd: number, runtimeId: number[], tabTitle: string) => void;
    importing?: boolean;
  } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="attach-overlay" onclick={onclose} onkeydown={(e) => { if (e.key === 'Escape') onclose(); }}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="attach-modal" onclick={(e) => e.stopPropagation()}>
    <div class="attach-header">
      <h3>{t.terminalList.attachExternal}</h3>
      <button class="close-btn" onclick={onclose}><Icon name="close" size={MODAL_CLOSE_ICON_SIZE} /></button>
    </div>
    <div class="attach-body">
      {#if importing}
        <p class="attach-loading">{t.attach.importing}</p>
      {:else if externalTerminals.length === 0 || externalTerminals.every(e => e.tabs.length === 0)}
        <p class="attach-empty">No external terminals found</p>
      {:else}
        {#each externalTerminals as ext (ext.hwnd)}
          {#if ext.tabs.length > 0}
            <div class="attach-window">
              <div class="attach-window-title">{ext.windowTitle}</div>
              {#each ext.tabs as tab (tab.runtimeId.join('-'))}
                <button class="attach-tab-btn" onclick={() => onattach(ext.hwnd, tab.runtimeId, tab.title)}>
                  <Icon name="terminal" size={MODAL_ITEM_ICON_SIZE} />
                  <span>{tab.title}</span>
                  <span class="attach-action"><Icon name="link" size={MODAL_ACTION_ICON_SIZE} /></span>
                </button>
              {/each}
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .attach-overlay {
    position: fixed;
    inset: 0;
    background: var(--termi-overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .attach-modal {
    background: var(--termi-bg-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-md, 8px);
    width: 400px;
    max-height: 500px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .attach-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--termi-spacing-md, 16px);
    border-bottom: 1px solid var(--termi-border);
  }

  .attach-header h3 {
    margin: 0;
    font-size: var(--termi-font-size-md, 14px);
    color: var(--termi-text-primary);
  }

  .close-btn {
    font-size: var(--termi-font-size-xs, 12px);
    padding: var(--termi-spacing-xxs, 2px) var(--termi-spacing-xs, 4px);
    border-radius: var(--termi-radius-sm, 3px);
    background: none;
    border: none;
    cursor: pointer;
    transition: opacity 0.15s, background-color 0.15s;
    color: var(--termi-text-secondary);
  }

  .close-btn:hover {
    background-color: var(--termi-bg-primary);
    color: var(--termi-danger);
  }

  .close-btn:focus-visible {
    opacity: 1;
    outline: 2px solid var(--termi-accent);
    outline-offset: 2px;
  }

  .attach-body {
    padding: var(--termi-spacing-sm, 8px);
    overflow-y: auto;
    max-height: 400px;
  }

  .attach-empty {
    color: var(--termi-text-secondary);
    text-align: center;
    font-size: var(--termi-font-size-sm, 13px);
    padding: var(--termi-spacing-lg, 24px);
  }

  .attach-loading {
    color: var(--termi-accent);
    text-align: center;
    font-size: var(--termi-font-size-sm, 13px);
    padding: var(--termi-spacing-lg, 24px);
  }

  .attach-window {
    margin-bottom: var(--termi-spacing-sm, 8px);
  }

  .attach-window-title {
    font-size: var(--termi-font-size-xs, 11px);
    color: var(--termi-text-secondary);
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-sm, 8px);
    font-weight: 600;
  }

  .attach-tab-btn {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-sm, 8px);
    padding: var(--termi-spacing-sm, 8px) var(--termi-spacing-md-sm, 12px);
    background: none;
    border: none;
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 13px);
    cursor: pointer;
    transition: background-color 0.15s;
    text-align: left;
  }

  .attach-tab-btn:hover {
    background-color: var(--termi-bg-surface);
  }

  .attach-tab-btn:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .attach-action {
    margin-left: auto;
    color: var(--termi-accent);
    opacity: 0;
    transition: opacity 0.15s;
  }

  .attach-tab-btn:hover .attach-action {
    opacity: 1;
  }
</style>
