<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog';
  import type { Project } from '$lib/types';
  import { deleteProject } from '$lib/stores/projects.svelte';
  import { launchTerminal, terminalsStore } from '$lib/stores/terminals.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { i18n } from '$lib/i18n/index.svelte';
  import { ACTION_ICON_SIZE } from '$lib/constants';

  const t = $derived(i18n.t);

  let {
    project,
    onSelect,
    onEdit,
    isDragging = false,
    dropPosition = null,
    onDragStart,
    onDragEnd,
  }: {
    project: Project;
    onSelect: (project: Project) => void;
    onEdit: (project: Project) => void;
    isDragging?: boolean;
    dropPosition?: 'before' | 'after' | null;
    onDragStart?: (projectId: string, e: DragEvent) => void;
    onDragEnd?: () => void;
  } = $props();

  let deleteError = $state<string | null>(null);
  let launchError = $state<string | null>(null);

  let terminalCount = $derived(
    terminalsStore.terminals.filter(t => t.projectId === project.id && t.status === 'running').length
  );
  let isRunning = $derived(terminalCount > 0);

  async function handleDelete() {
    const confirmed = await confirm(
      t.projectItem.deleteConfirm(project.name),
      { title: t.projectItem.deleteTitle, kind: 'warning' },
    );
    if (confirmed) {
      try {
        deleteError = null;
        await deleteProject(project.id);
      } catch (e) {
        deleteError = String(e);
      }
    }
  }

  async function handleLaunch() {
    launchError = null;
    try {
      await launchTerminal(project.id);
    } catch (e) {
      launchError = String(e);
    }
  }

  function handleDragStart(e: DragEvent) {
    onDragStart?.(project.id, e);
  }

  function handleDragEnd() {
    onDragEnd?.();
  }
</script>

{#if deleteError}
  <p class="error-message">{deleteError}</p>
{/if}
{#if launchError}
  <p class="error-message">{launchError}</p>
{/if}
<div
  class="project-item"
  class:running={isRunning}
  class:dragging={isDragging}
  class:drop-before={dropPosition === 'before'}
  class:drop-after={dropPosition === 'after'}
  role="option"
  tabindex="0"
  aria-selected="false"
  draggable="true"
  ondragstart={handleDragStart}
  ondragend={handleDragEnd}
  aria-grabbed={isDragging}
  aria-dropeffect="move"
  data-project-id={project.id}
>
  <div
    class="project-info"
    role="button"
    tabindex="0"
    onclick={() => onSelect(project)}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(project); } }}
  >
    <span class="project-name">{project.name}</span>
    <span class="project-path">{project.path}</span>
  </div>
  <div class="project-actions">
    {#if terminalCount > 0}
      <span class="terminal-count" title={t.projectItem.runningCount(terminalCount)}>{terminalCount}</span>
    {/if}
    <button
      class="action-btn launch-btn"
      onclick={handleLaunch}
      title={t.projectItem.launchTerminal}
      disabled={terminalsStore.launching}
    >
      <Icon name="play" size={ACTION_ICON_SIZE} />
    </button>
    <button
      class="action-btn edit-btn"
      onclick={() => onEdit(project)}
      title={t.projectItem.edit}
    >
      <Icon name="edit" size={ACTION_ICON_SIZE} />
    </button>
    <button
      class="action-btn delete-btn"
      onclick={handleDelete}
      title={t.projectItem.delete}
    >
      <Icon name="trash" size={ACTION_ICON_SIZE} />
    </button>
  </div>
</div>

<style>
  .error-message {
    padding: 2px 12px;
  }

  .project-item {
    display: flex;
    align-items: center;
    border-radius: var(--termi-radius-md, 6px);
    transition: background-color 0.15s;
  }

  .project-item.running {
    border-left: 3px solid var(--termi-accent);
  }

  .project-item:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .project-item:hover {
    background-color: var(--termi-bg-surface);
  }

  .project-item.dragging {
    opacity: 0.5;
    cursor: grabbing;
  }

  .project-item.drop-before {
    border-top: 2px solid var(--termi-accent);
  }

  .project-item.drop-after {
    border-bottom: 2px solid var(--termi-accent);
  }

  .project-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 8px 12px;
    color: var(--termi-text-primary);
    min-width: 0;
    cursor: pointer;
  }

  .project-info:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .project-name {
    font-size: var(--termi-font-size-sm, 14px);
    font-weight: 500;
  }

  .project-path {
    font-size: var(--termi-font-size-xs, 12px);
    color: var(--termi-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }

  .project-actions {
    display: none;
    align-items: center;
    gap: 2px;
    padding-right: 8px;
  }

  .project-item:hover .project-actions {
    display: flex;
  }

  .action-btn {
    padding: 4px 6px;
    background: none;
    border: none;
    border-radius: var(--termi-radius-sm, 4px);
    font-size: var(--termi-font-size-sm, 13px);
    color: var(--termi-text-secondary);
    transition: background-color 0.15s, color 0.15s;
  }

  .action-btn:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .launch-btn:hover {
    background-color: var(--termi-bg-primary);
    color: var(--termi-accent);
  }

  .edit-btn:hover {
    background-color: var(--termi-bg-primary);
    color: var(--termi-accent);
  }

  .delete-btn:hover {
    background-color: var(--termi-bg-primary);
    color: var(--termi-danger);
  }

  .terminal-count {
    font-size: var(--termi-font-size-xs, 11px);
    background-color: var(--termi-accent);
    color: var(--termi-bg-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--termi-icon-size-sm, 16px);
    height: var(--termi-icon-size-sm, 16px);
    border-radius: var(--termi-radius-full, 50%);
    font-weight: 600;
    flex-shrink: 0;
  }
</style>
