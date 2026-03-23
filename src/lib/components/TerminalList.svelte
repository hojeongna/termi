<script lang="ts">
  import { terminalsStore, focusTerminal, closeTerminal, launchTerminal, renameTerminal, toggleNotification, discoverExternalTerminals, attachTerminal } from '$lib/stores/terminals.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AttachModal from '$lib/components/AttachModal.svelte';
  import type { TerminalInstance, ExternalTerminalInfo } from '$lib/types';
  import { i18n } from '$lib/i18n/index.svelte';
  import { catchError } from '$lib/utils/error';

  import { EDIT_INPUT_MIN_WIDTH_PX, EDIT_INPUT_PADDING_PX, EDIT_INPUT_MAX_WIDTH_PX, ALL_PROJECTS_ID, MS_PER_SECOND } from '$lib/constants';

  const t = $derived(i18n.t);

  let error = $state<string | null>(null);
  function setError(msg: string | null) { error = msg; }
  let editingId = $state<string | null>(null);
  let editingName = $state('');
  let inputWidth = $state(0);
  let showAttachModal = $state(false);
  let externalTerminals = $state<ExternalTerminalInfo[]>([]);
  let attachLoading = $state(false);

  function autofocus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function formatTime(timestampStr: string): string {
    try {
      const secs = parseInt(timestampStr.replace('Z', ''), 10);
      if (isNaN(secs)) return '';
      const date = new Date(secs * MS_PER_SECOND);
      return date.toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' });
    } catch {
      return '';
    }
  }

  function statusLabel(activity: string): string {
    switch (activity) {
      case 'active': return t.terminalList.statusActive;
      case 'idle': return t.terminalList.statusIdle;
      default: return '';
    }
  }

  async function handleFocus(terminalId: string) {
    await catchError(setError, () => focusTerminal(terminalId));
  }

  async function handleClose(terminalId: string, e: MouseEvent) {
    e.stopPropagation();
    await catchError(setError, () => closeTerminal(terminalId));
  }

  function startEditing(terminal: TerminalInstance) {
    editingId = terminal.id;
    editingName = terminal.terminalName;
  }

  async function finishEditing(terminalId: string) {
    const trimmed = editingName.trim();
    if (trimmed && trimmed !== terminalsStore.activeProjectTerminals.find(t => t.id === terminalId)?.terminalName) {
      await catchError(setError, () => renameTerminal(terminalId, trimmed));
    }
    editingId = null;
  }

  function handleEditKeydown(e: KeyboardEvent, terminalId: string) {
    e.stopPropagation();
    if (e.key === 'Enter') {
      finishEditing(terminalId);
    } else if (e.key === 'Escape') {
      editingId = null;
    }
  }

  async function handleToggleNotification(terminalId: string, e: MouseEvent) {
    e.stopPropagation();
    await catchError(setError, () => toggleNotification(terminalId));
  }

  async function handleDiscover() {
    attachLoading = true;
    await catchError(setError, async () => {
      externalTerminals = await discoverExternalTerminals();
      showAttachModal = true;
    });
    attachLoading = false;
  }

  async function handleAttach(hwnd: number, runtimeId: number[], tabTitle: string) {
    const projectId = terminalsStore.activeProjectId;
    if (!projectId) return;
    await catchError(setError, async () => {
      await attachTerminal(hwnd, runtimeId, tabTitle, projectId);
      // 연결 후 목록 새로고침
      externalTerminals = await discoverExternalTerminals();
      if (externalTerminals.every(e => e.tabs.length === 0)) {
        showAttachModal = false;
      }
    });
  }

  async function handleAddTerminal() {
    const projectId = terminalsStore.activeProjectId;
    if (!projectId) return;
    await catchError(setError, () => launchTerminal(projectId));
  }
</script>

{#if terminalsStore.activeProjectTerminals.length > 0}
<div class="terminal-list">
  {#each terminalsStore.activeProjectTerminals as terminal, i (terminal.id)}
    <div
      class="terminal-row"
      class:active={terminal.id === terminalsStore.activeTabId}
      role="button"
      tabindex="0"
      onclick={() => handleFocus(terminal.id)}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleFocus(terminal.id); }}
    >
      <div class="terminal-info">
        <StatusBadge status={terminal.activity} monitored={terminal.monitored} />
        {#if editingId === terminal.id}
          <span class="input-sizer" bind:offsetWidth={inputWidth}>{editingName || ' '}</span>
          <!-- The wrapping div handles keyboard events from the inner input; svelte-ignore is needed because the outer div is not itself an interactive element -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <input
            class="terminal-name-input"
            style="width: {Math.max(EDIT_INPUT_MIN_WIDTH_PX, Math.min(inputWidth + EDIT_INPUT_PADDING_PX, EDIT_INPUT_MAX_WIDTH_PX))}px"
            bind:value={editingName}
            use:autofocus
            onclick={(e) => e.stopPropagation()}
            onblur={() => finishEditing(terminal.id)}
            onkeydown={(e) => handleEditKeydown(e, terminal.id)}
          />
        {:else}
          <span class="terminal-name">{#if terminalsStore.activeProjectId === ALL_PROJECTS_ID}<span class="project-tag">{terminal.projectName}</span>{/if}{terminal.terminalName}{#if terminal.attached}<span class="attached-badge" title={t.terminalList.attached}><Icon name="link" size={10} /></span>{/if}</span>
          <button
            class="icon-btn edit-btn"
            tabindex="-1"
            title={t.terminalList.editName}
            onclick={(e) => { e.stopPropagation(); startEditing(terminal); }}
          ><Icon name="edit" size={12} /></button>
        {/if}
        {#if terminal.monitored}<span class="terminal-status-label">{statusLabel(terminal.activity)}</span>{/if}
      </div>
      <div class="terminal-meta">
        <span class="terminal-time">{formatTime(terminal.lastIdleAt ?? terminal.launchedAt)}</span>
        <button
          class="icon-btn notification-btn"
          class:muted={!terminal.notificationEnabled}
          tabindex="-1"
          title={terminal.notificationEnabled ? t.terminalList.notificationOff : t.terminalList.notificationOn}
          onclick={(e) => handleToggleNotification(terminal.id, e)}
        ><Icon name={terminal.notificationEnabled ? 'bell' : 'bell-off'} size={14} /></button>
        <button
          class="close-btn"
          tabindex="-1"
          onclick={(e) => handleClose(terminal.id, e)}
        ><Icon name="close" size={12} /></button>
      </div>
    </div>
  {/each}

  {#if terminalsStore.activeProjectId !== ALL_PROJECTS_ID}
  <button class="add-terminal-btn" onclick={handleAddTerminal} disabled={terminalsStore.launching}>
    <Icon name="plus" size={12} /> {t.terminalList.newTerminal}
  </button>

  <button class="add-terminal-btn attach-btn" onclick={handleDiscover} disabled={attachLoading}>
    <Icon name="link" size={12} /> {t.terminalList.attachExternal}
  </button>
  {/if}

  {#if error}
    <p class="list-error">{error}</p>
  {/if}
</div>
{/if}

{#if showAttachModal}
<AttachModal
  {externalTerminals}
  onclose={() => showAttachModal = false}
  onattach={handleAttach}
/>
{/if}

<style>
  .terminal-list {
    padding: var(--termi-spacing-sm, 8px);
    border-bottom: 1px solid var(--termi-border);
    background-color: var(--termi-bg-primary);
  }

  .terminal-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--termi-spacing-sm, 8px) var(--termi-spacing-md-sm, 12px);
    background: none;
    border: none;
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-sm, 13px);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .terminal-row:hover {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
  }

  .terminal-row:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .terminal-row.active {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
    border-left: 2px solid var(--termi-accent);
  }

  .terminal-info {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-sm, 8px);
  }

  .terminal-name {
    font-weight: 500;
  }

  .project-tag {
    font-size: var(--termi-font-size-xs, 11px);
    color: var(--termi-accent);
    margin-right: var(--termi-spacing-xs, 4px);
    font-weight: 600;
  }

  .input-sizer {
    position: absolute;
    visibility: hidden;
    white-space: pre;
    font-size: var(--termi-font-size-sm, 13px);
    font-weight: 500;
    padding: var(--termi-spacing-xxs, 1px) var(--termi-spacing-xs, 4px);
  }

  .terminal-name-input {
    background: var(--termi-bg-primary);
    border: 1px solid var(--termi-accent);
    border-radius: var(--termi-radius-sm, 3px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 13px);
    font-weight: 500;
    padding: var(--termi-spacing-xxs, 1px) var(--termi-spacing-xs, 4px);
    min-width: 60px;
    max-width: 300px;
    outline: none;
  }

  .terminal-name-input:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .terminal-status-label {
    font-size: var(--termi-font-size-xs, 11px);
    color: var(--termi-text-secondary);
  }

  .terminal-meta {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-sm, 8px);
  }

  .terminal-time {
    font-size: var(--termi-font-size-xs, 11px);
    color: var(--termi-text-secondary);
  }

  .icon-btn {
    font-size: var(--termi-font-size-xs, 11px);
    padding: var(--termi-spacing-xxs, 2px) var(--termi-spacing-xs, 4px);
    border-radius: var(--termi-radius-sm, 3px);
    background: none;
    border: none;
    color: var(--termi-text-secondary);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .terminal-row:hover .icon-btn {
    opacity: 1;
    color: var(--termi-text-primary);
  }

  .notification-btn.muted {
    opacity: 0.5;
  }

  .terminal-row:hover .notification-btn.muted {
    opacity: 0.7;
  }

  .close-btn {
    font-size: var(--termi-font-size-xs, 12px);
    padding: var(--termi-spacing-xxs, 2px) var(--termi-spacing-xs, 4px);
    border-radius: var(--termi-radius-sm, 3px);
    background: none;
    border: none;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background-color 0.15s;
    color: var(--termi-text-secondary);
  }

  .terminal-row:hover .close-btn {
    opacity: 1;
  }

  .close-btn:hover {
    background-color: var(--termi-bg-primary);
    color: var(--termi-danger);
  }

  .icon-btn:focus-visible {
    opacity: 1;
    outline: 2px solid var(--termi-accent);
    outline-offset: 2px;
  }

  .close-btn:focus-visible {
    opacity: 1;
    outline: 2px solid var(--termi-accent);
    outline-offset: 2px;
  }

  .add-terminal-btn {
    width: 100%;
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-md-sm, 12px);
    margin-top: var(--termi-spacing-xs, 4px);
    background: none;
    border: 1px dashed var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-xs, 12px);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  }

  .add-terminal-btn:hover {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
    border-color: var(--termi-text-secondary);
  }

  .add-terminal-btn:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .list-error {
    font-size: var(--termi-font-size-xs, 11px);
    color: var(--termi-danger);
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-md-sm, 12px);
    margin: 0;
  }

  .attached-badge {
    margin-left: var(--termi-spacing-xs, 4px);
    color: var(--termi-accent);
    opacity: 0.7;
  }

  .attach-btn {
    margin-top: var(--termi-spacing-sm, 8px);
  }
</style>
