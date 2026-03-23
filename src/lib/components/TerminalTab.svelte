<script lang="ts">
  import type { TerminalInstance } from '$lib/types';
  import { focusTerminal, closeTerminal } from '$lib/stores/terminals.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { catchError } from '$lib/utils/error';

  let { terminal, isActive }: {
    terminal: TerminalInstance;
    isActive: boolean;
  } = $props();

  let error = $state<string | null>(null);
  function setError(msg: string | null) { error = msg; }

  async function handleClick() {
    await catchError(setError, () => focusTerminal(terminal.id));
  }

  async function handleClose(e: MouseEvent) {
    e.stopPropagation();
    await catchError(setError, () => closeTerminal(terminal.id));
  }
</script>

<button
  class="terminal-tab"
  class:active={isActive}
  onclick={handleClick}
>
  <StatusBadge status={terminal.activity} monitored={terminal.monitored} />
  <span class="tab-name">{terminal.projectName}</span>
  <span class="close-btn" role="button" tabindex="-1" onclick={handleClose}><Icon name="close" size={10} /></span>
</button>
{#if error}
  <span class="tab-error">{error}</span>
{/if}

<style>
  .terminal-tab {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-xs, 6px);
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-md, 12px);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-md, 14px);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    white-space: nowrap;
  }

  .terminal-tab:hover {
    color: var(--termi-text-primary);
  }

  .terminal-tab:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .terminal-tab.active {
    color: var(--termi-text-primary);
    border-bottom-color: var(--termi-accent);
  }

  .tab-name {
    font-weight: 500;
  }

  .close-btn {
    font-size: var(--termi-font-size-xs, 12px);
    padding: 2px var(--termi-spacing-xs, 4px);
    border-radius: var(--termi-radius-sm, 3px);
    background: none;
    border: none;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background-color 0.15s;
  }

  .terminal-tab:hover .close-btn {
    opacity: 1;
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

  .tab-error {
    font-size: var(--termi-font-size-xs, 12px);
    color: var(--termi-danger);
    padding: 2px var(--termi-spacing-xs, 4px);
  }
</style>
