<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '$lib/api/invoke';
  import type { DebugLogEntry } from '$lib/types';
  import { i18n } from '$lib/i18n/index.svelte';
  import { EVENT_DEBUG_LOG_UPDATED, COPY_FEEDBACK_DURATION_MS } from '$lib/constants';

  const t = $derived(i18n.t);

  let debugLogs = $state.raw<DebugLogEntry[]>([]);
  let showDebugLog = $state(false);
  let autoRefresh = $state(false);
  let autoRefreshUnlisten: (() => void) | null = null;
  let logContainer: HTMLPreElement | undefined = $state();
  let copied = $state(false);
  let debugError = $state('');

  function formatLogTime(timestamp: number): string {
    const d = new Date(timestamp);
    const hh = String(d.getHours()).padStart(2, '0');
    const mm = String(d.getMinutes()).padStart(2, '0');
    const ss = String(d.getSeconds()).padStart(2, '0');
    const ms = String(d.getMilliseconds()).padStart(3, '0');
    return `${hh}:${mm}:${ss}.${ms}`;
  }

  function formatLogsAsText(): string {
    return debugLogs
      .map(e => `[${formatLogTime(e.timestamp)}] [${e.category}] ${e.message}`)
      .join('\n');
  }

  async function loadDebugLogs() {
    try {
      debugLogs = await invoke<DebugLogEntry[]>('get_debug_logs');
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
        }
      });
    } catch (err) {
      debugError = t.settings.debug.loadFailed + String(err);
    }
  }

  async function clearDebugLogs() {
    try {
      await invoke<void>('clear_debug_logs');
      debugLogs = [];
    } catch (err) {
      debugError = t.settings.debug.clearFailed + String(err);
    }
  }

  async function copyDebugLogs() {
    try {
      await navigator.clipboard.writeText(formatLogsAsText());
      copied = true;
      setTimeout(() => { copied = false; }, COPY_FEEDBACK_DURATION_MS);
    } catch {
      const text = formatLogsAsText();
      const textarea = document.createElement('textarea');
      textarea.value = text;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      copied = true;
      setTimeout(() => { copied = false; }, COPY_FEEDBACK_DURATION_MS);
    }
  }

  async function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    if (autoRefresh) {
      if (autoRefreshUnlisten) {
        autoRefreshUnlisten();
        autoRefreshUnlisten = null;
      }
      loadDebugLogs();
      autoRefreshUnlisten = await listen<void>(EVENT_DEBUG_LOG_UPDATED, () => {
        loadDebugLogs();
      });
    } else if (autoRefreshUnlisten) {
      autoRefreshUnlisten();
      autoRefreshUnlisten = null;
    }
  }

  function handleToggleDebugLog() {
    showDebugLog = !showDebugLog;
    if (showDebugLog) {
      loadDebugLogs();
    } else {
      autoRefresh = false;
      if (autoRefreshUnlisten) {
        autoRefreshUnlisten();
        autoRefreshUnlisten = null;
      }
    }
  }

  $effect(() => {
    return () => {
      if (autoRefreshUnlisten) {
        autoRefreshUnlisten();
      }
    };
  });
</script>

<div class="settings-section">
  <h4>
    <button class="debug-toggle" onclick={handleToggleDebugLog}>
      <span class="debug-arrow" class:open={showDebugLog}>&#9654;</span>
      {t.settings.debug.title}
    </button>
  </h4>

  {#if showDebugLog}
    <div class="debug-toolbar">
      <button class="debug-btn" onclick={loadDebugLogs}>{t.settings.debug.refresh}</button>
      <button class="debug-btn" class:active={autoRefresh} onclick={toggleAutoRefresh}>
        {autoRefresh ? t.settings.debug.autoRefreshOn : t.settings.debug.autoRefreshOff}
      </button>
      <button class="debug-btn" onclick={copyDebugLogs}>
        {copied ? t.settings.debug.copied : t.settings.debug.copy}
      </button>
      <button class="debug-btn danger" onclick={clearDebugLogs}>{t.settings.debug.clear}</button>
      <span class="debug-count">{t.settings.debug.count(debugLogs.length)}</span>
    </div>

    {#if debugError}
      <p class="error-message">{debugError}</p>
    {/if}

    <pre class="debug-log-area" bind:this={logContainer}>{#if debugLogs.length === 0}<span class="debug-empty">{t.settings.debug.empty}</span>{:else}{#each debugLogs as entry, i (entry.timestamp + '-' + i)}[{formatLogTime(entry.timestamp)}] [{entry.category}] {entry.message}
{/each}{/if}</pre>
  {/if}
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

  .error-message {
    margin-top: var(--termi-spacing-md-sm, 12px);
  }

  .debug-toggle {
    background: none;
    border: none;
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-sm, 14px);
    font-weight: 600;
    cursor: pointer;
    padding: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .debug-toggle:hover {
    color: var(--termi-text-primary);
  }

  .debug-toggle:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .debug-arrow {
    font-size: var(--termi-font-size-xs, 10px);
    transition: transform 0.15s ease;
    display: inline-block;
  }

  .debug-arrow.open {
    transform: rotate(90deg);
  }

  .debug-toolbar {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: var(--termi-spacing-sm, 8px);
    flex-wrap: wrap;
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

  .debug-btn.active {
    background-color: var(--termi-accent);
    color: var(--termi-text-primary);
    border-color: var(--termi-accent);
  }

  .debug-btn.danger:hover {
    border-color: var(--termi-danger);
    color: var(--termi-danger);
  }

  .debug-count {
    font-size: var(--termi-font-size-xs, 12px);
    color: var(--termi-text-secondary);
    margin-left: auto;
  }

  .debug-log-area {
    /* Component-local CSS custom property definitions (not available globally) */
    --termi-debug-log-max-height: 360px;
    --termi-font-size-xxs: 11px;

    background-color: var(--termi-bg-secondary);
    color: var(--termi-text-secondary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    padding: var(--termi-spacing-sm, 8px);
    font-family: var(--termi-font-mono, 'Cascadia Code', monospace);
    font-size: var(--termi-font-size-xxs);
    line-height: 1.5;
    max-height: var(--termi-debug-log-max-height);
    overflow-y: auto;
    overflow-x: auto;
    white-space: pre;
    margin: 0;
    word-break: keep-all;
  }

  .debug-empty {
    color: var(--termi-text-secondary);
    opacity: 0.6;
    font-style: italic;
  }
</style>
