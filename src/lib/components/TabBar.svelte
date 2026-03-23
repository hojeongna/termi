<script lang="ts">
  import { terminalsStore } from '$lib/stores/terminals.svelte';
  import { settingsStore, savePartialSettings } from '$lib/stores/settings.svelte';
  import { i18n } from '$lib/i18n/index.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { ALL_PROJECTS_ID } from '$lib/constants';

  const t = $derived(i18n.t);
  const isAlwaysOnTop = $derived(settingsStore.settings?.alwaysOnTop ?? false);

  function toggleAlwaysOnTop() {
    savePartialSettings({ alwaysOnTop: !isAlwaysOnTop });
  }

  // --- Drag and Drop state ---
  let draggedProjectId = $state<string | null>(null);
  let dropIndicator = $state<{ projectId: string; side: 'left' | 'right' } | null>(null);
  let dragOverRaf: number | null = null;

  function handleDragStart(e: DragEvent, projectId: string) {
    if (projectId === ALL_PROJECTS_ID || !e.dataTransfer) return;
    draggedProjectId = projectId;
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', projectId);
  }

  function handleDragOver(e: DragEvent, projectId: string) {
    if (!draggedProjectId || projectId === ALL_PROJECTS_ID || projectId === draggedProjectId) {
      return;
    }
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }

    if (dragOverRaf !== null) return;
    const clientX = e.clientX;
    const currentTarget = e.currentTarget;
    dragOverRaf = requestAnimationFrame(() => {
      dragOverRaf = null;
      if (!(currentTarget instanceof HTMLElement)) return;
      const rect = currentTarget.getBoundingClientRect();
      const midX = rect.left + rect.width / 2;
      const side: 'left' | 'right' = clientX < midX ? 'left' : 'right';
      dropIndicator = { projectId, side };
    });
  }

  function handleDragLeave(e: DragEvent, projectId: string) {
    const relatedTarget = e.relatedTarget;
    const currentTarget = e.currentTarget;
    if (!(currentTarget instanceof HTMLElement)) return;
    if (relatedTarget instanceof Node && currentTarget.contains(relatedTarget)) return;

    if (dropIndicator?.projectId === projectId) {
      dropIndicator = null;
    }
  }

  function handleDrop(e: DragEvent, targetProjectId: string) {
    e.preventDefault();
    if (!draggedProjectId || targetProjectId === ALL_PROJECTS_ID || draggedProjectId === targetProjectId) {
      cleanupDrag();
      return;
    }

    const projects = terminalsStore.orderedProjectsWithTerminals;
    const currentOrder = projects.map(p => p.projectId);

    const draggedIdx = currentOrder.indexOf(draggedProjectId);
    const targetIdx = currentOrder.indexOf(targetProjectId);

    if (draggedIdx === -1 || targetIdx === -1) {
      cleanupDrag();
      return;
    }

    // Remove dragged item from current position
    const newOrder = currentOrder.filter(id => id !== draggedProjectId);

    // Calculate insert index based on indicator side
    let insertIdx: number;
    const targetIdxInNew = newOrder.indexOf(targetProjectId);
    if (dropIndicator?.side === 'right') {
      insertIdx = targetIdxInNew + 1;
    } else {
      insertIdx = targetIdxInNew;
    }

    newOrder.splice(insertIdx, 0, draggedProjectId);
    terminalsStore.tabOrder = newOrder;

    cleanupDrag();
  }

  function handleDragEnd() {
    cleanupDrag();
  }

  function cleanupDrag() {
    if (dragOverRaf !== null) {
      cancelAnimationFrame(dragOverRaf);
      dragOverRaf = null;
    }
    draggedProjectId = null;
    dropIndicator = null;
  }
</script>

{#if terminalsStore.projectsWithTerminals.length > 0}
<div class="tab-bar">
  <button
    class="project-tab"
    class:active={terminalsStore.activeProjectId === ALL_PROJECTS_ID}
    onclick={() => terminalsStore.activeProjectId = ALL_PROJECTS_ID}
  >
    <span class="tab-name">{t.tabBar.all}</span>
    <span class="tab-count">{terminalsStore.terminals.length}</span>
  </button>
  {#each terminalsStore.orderedProjectsWithTerminals as proj (proj.projectId)}
    <button
      class="project-tab"
      class:active={proj.projectId === terminalsStore.activeProjectId}
      class:dragging={draggedProjectId === proj.projectId}
      class:drop-left={dropIndicator?.projectId === proj.projectId && dropIndicator?.side === 'left'}
      class:drop-right={dropIndicator?.projectId === proj.projectId && dropIndicator?.side === 'right'}
      draggable="true"
      aria-roledescription="sortable"
      onclick={() => terminalsStore.activeProjectId = proj.projectId}
      ondragstart={(e) => handleDragStart(e, proj.projectId)}
      ondragover={(e) => handleDragOver(e, proj.projectId)}
      ondragleave={(e) => handleDragLeave(e, proj.projectId)}
      ondrop={(e) => handleDrop(e, proj.projectId)}
      ondragend={handleDragEnd}
    >
      <span class="tab-name">{proj.projectName}</span>
      {#if proj.terminalCount > 1}
        <span class="tab-count">{proj.terminalCount}</span>
      {/if}
    </button>
  {/each}
  <button
    class="pin-toggle"
    class:active={isAlwaysOnTop}
    onclick={toggleAlwaysOnTop}
  >
    <Icon name="pin" size={14} />
    <span class="pin-tooltip">{t.tabBar.pinTooltip}</span>
  </button>
</div>
{/if}

<style>
  .tab-bar {
    display: flex;
    background-color: var(--termi-bg-secondary);
    border-bottom: 1px solid var(--termi-border);
    padding: 0 var(--termi-spacing-sm, 8px);
    gap: var(--termi-spacing-xxs, 2px);
    min-height: var(--termi-tab-bar-height, 36px);
    align-items: flex-end;
  }

  .project-tab {
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-xs, 4px);
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-sm, 12px);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-sm, 13px);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, opacity 0.15s;
    white-space: nowrap;
    position: relative;
  }

  .project-tab:hover {
    color: var(--termi-text-primary);
  }

  .project-tab:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .project-tab.active {
    color: var(--termi-text-primary);
    border-bottom-color: var(--termi-accent);
  }

  /* Drag and Drop styles */
  .project-tab.dragging {
    opacity: 0.5;
  }

  .project-tab.drop-left::before {
    content: '';
    position: absolute;
    left: -2px;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background-color: var(--termi-accent);
    border-radius: 2px;
    pointer-events: none;
  }

  .project-tab.drop-right::after {
    content: '';
    position: absolute;
    right: -2px;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background-color: var(--termi-accent);
    border-radius: 2px;
    pointer-events: none;
  }

  .project-tab[draggable="true"] {
    cursor: grab;
  }

  .project-tab[draggable="true"]:active {
    cursor: grabbing;
  }

  .tab-name {
    font-weight: 500;
  }

  .tab-count {
    font-size: var(--termi-font-size-xs, 11px);
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-secondary);
    padding: 0 var(--termi-spacing-xs, 5px);
    border-radius: var(--termi-radius-md, 8px);
    min-width: var(--termi-icon-size-sm, 16px);
    text-align: center;
    line-height: var(--termi-icon-size-sm, 16px);
  }

  .project-tab.active .tab-count {
    background-color: var(--termi-accent);
    color: var(--termi-bg-primary);
  }

  .pin-toggle {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: auto;
    padding: var(--termi-spacing-xs, 6px);
    background: none;
    border: none;
    cursor: pointer;
    color: var(--termi-text-secondary);
    transition: color 0.15s;
    align-self: center;
  }

  .pin-toggle:hover {
    color: var(--termi-text-primary);
  }

  .pin-toggle:focus-visible {
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .pin-toggle.active {
    color: var(--termi-accent);
  }

  .pin-tooltip {
    position: absolute;
    top: calc(100% + var(--termi-spacing-xs, 4px));
    right: 0;
    padding: var(--termi-spacing-xs, 4px) var(--termi-spacing-sm, 8px);
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-xs, 11px);
    border-radius: var(--termi-radius-sm, 4px);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s;
    border: 1px solid var(--termi-border);
    z-index: 10;
  }

  .pin-toggle:hover .pin-tooltip {
    opacity: 1;
  }
</style>
