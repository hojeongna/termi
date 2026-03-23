<script lang="ts">
  import type { Project } from '$lib/types';
  import { projectsStore, loadProjects, reorderProjects } from '$lib/stores/projects.svelte';
  import { terminalsStore } from '$lib/stores/terminals.svelte';
  import ProjectItem from '$lib/components/ProjectItem.svelte';
  import ProjectForm from '$lib/components/ProjectForm.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { i18n } from '$lib/i18n/index.svelte';

  const t = $derived(i18n.t);
  let { onSettingsClick = () => {} }: { onSettingsClick?: () => void } = $props();

  let showForm = $state(false);
  let editingProject = $state<Project | undefined>(undefined);
  let error = $state<string | null>(null);
  const projects = $derived(projectsStore.projects);

  // Drag and Drop state
  let draggedProjectId = $state<string | null>(null);
  let dropTargetId = $state<string | null>(null);
  let dropPosition = $state<'before' | 'after' | null>(null);
  let dragOverRaf: number | null = null;

  $effect(() => {
    loadProjects().catch((err) => {
      console.error('Failed to load projects:', err);
      error = String(err);
    });
  });

  function handleAddProject() {
    editingProject = undefined;
    showForm = true;
  }

  function handleEditProject(project: Project) {
    editingProject = project;
    showForm = true;
  }

  function handleFormClose() {
    showForm = false;
    editingProject = undefined;
  }

  function handleSelectProject(project: Project) {
    terminalsStore.activeProjectId = project.id;
  }

  // --- Drag and Drop handlers ---

  function handleItemDragStart(projectId: string, e: DragEvent) {
    draggedProjectId = projectId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', projectId);
    }
  }

  function handleItemDragEnd() {
    if (dragOverRaf !== null) {
      cancelAnimationFrame(dragOverRaf);
      dragOverRaf = null;
    }
    draggedProjectId = null;
    dropTargetId = null;
    dropPosition = null;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (!draggedProjectId) return;
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }
    if (dragOverRaf !== null) return;
    const target = e.target;
    const clientY = e.clientY;
    dragOverRaf = requestAnimationFrame(() => {
      dragOverRaf = null;
      const el = target instanceof HTMLElement ? target.closest<HTMLElement>('[data-project-id]') : null;
      if (!el) {
        dropTargetId = null;
        dropPosition = null;
        return;
      }
      const targetId = el.dataset.projectId ?? null;
      if (targetId === draggedProjectId) {
        dropTargetId = null;
        dropPosition = null;
        return;
      }
      const rect = el.getBoundingClientRect();
      const midY = rect.top + rect.height / 2;
      const pos: 'before' | 'after' = clientY < midY ? 'before' : 'after';
      dropTargetId = targetId;
      dropPosition = pos;
    });
  }

  function handleDragLeave(e: DragEvent) {
    const relatedTarget = e.relatedTarget instanceof HTMLElement ? e.relatedTarget : null;
    const projectList = e.currentTarget;
    if (!(projectList instanceof HTMLElement)) return;
    if (!relatedTarget || !projectList.contains(relatedTarget)) {
      dropTargetId = null;
      dropPosition = null;
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    if (!draggedProjectId || !dropTargetId || !dropPosition) {
      handleItemDragEnd();
      return;
    }

    // Build the new order
    const currentIds = projects.map((p) => p.id);
    const draggedIndex = currentIds.indexOf(draggedProjectId);
    if (draggedIndex === -1) {
      handleItemDragEnd();
      return;
    }

    // Remove dragged item from the list
    const reordered = currentIds.filter((id) => id !== draggedProjectId);

    // Find the target index in the filtered list
    let targetIndex = reordered.indexOf(dropTargetId);
    if (targetIndex === -1) {
      handleItemDragEnd();
      return;
    }

    // Insert at the correct position
    if (dropPosition === 'after') {
      targetIndex += 1;
    }
    reordered.splice(targetIndex, 0, draggedProjectId);

    // Reset DnD state
    handleItemDragEnd();

    // Persist the new order
    reorderProjects(reordered).catch((err) => {
      console.error('Failed to reorder projects:', err);
      error = String(err);
    });
  }
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <h2>{t.sidebar.projectList}</h2>
  </div>

  {#if showForm}
    {#key editingProject?.id}
      <ProjectForm editProject={editingProject} onClose={handleFormClose} />
    {/key}
  {/if}

  {#if error}
    <p class="error-message">{error}</p>
  {/if}

  <div
    class="project-list"
    role="listbox"
    tabindex="0"
    aria-label={t.sidebar.projectList}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    {#if projects.length === 0}
      <p class="empty-message">{t.sidebar.noProjects}</p>
    {:else}
      {#each projects as project (project.id)}
        <ProjectItem
          {project}
          onSelect={handleSelectProject}
          onEdit={handleEditProject}
          isDragging={draggedProjectId === project.id}
          dropPosition={dropTargetId === project.id ? dropPosition : null}
          onDragStart={handleItemDragStart}
          onDragEnd={handleItemDragEnd}
        />
      {/each}
    {/if}
  </div>

  <div class="sidebar-footer">
    <button class="add-project-btn" onclick={handleAddProject}>
      <Icon name="plus" size={14} /> {t.sidebar.addProject}
    </button>
    <button class="settings-btn" onclick={onSettingsClick}>
      <Icon name="settings" size={14} /> {t.sidebar.settings}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--termi-sidebar-width);
    height: 100%;
    background-color: var(--termi-bg-secondary);
    border-right: 1px solid var(--termi-border);
    display: flex;
    flex-direction: column;
  }

  .sidebar-header {
    padding: var(--termi-spacing-md, 16px);
    border-bottom: 1px solid var(--termi-border);
  }

  .sidebar-header h2 {
    font-size: var(--termi-font-size-md, 16px);
    font-weight: 600;
    color: var(--termi-text-primary);
  }

  .project-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--termi-spacing-sm, 8px);
  }

  .error-message {
    padding: 2px 12px;
  }

  .empty-message {
    color: var(--termi-text-secondary);
    text-align: center;
    padding: var(--termi-spacing-lg, 24px) var(--termi-spacing-md, 16px);
    font-size: var(--termi-font-size-sm, 13px);
  }

  .sidebar-footer {
    padding: var(--termi-spacing-md-sm, 12px);
    border-top: 1px solid var(--termi-border);
  }

  .add-project-btn {
    width: 100%;
    padding: var(--termi-spacing-sm, 8px) var(--termi-spacing-md, 16px);
    background-color: var(--termi-accent);
    color: var(--termi-bg-primary);
    border: none;
    border-radius: var(--termi-radius-md, 6px);
    font-weight: 500;
    transition: background-color 0.15s;
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-xs, 6px);
    justify-content: center;
  }

  .add-project-btn:hover {
    background-color: var(--termi-accent-hover);
  }

  .settings-btn {
    width: 100%;
    padding: var(--termi-spacing-sm, 8px) var(--termi-spacing-md, 16px);
    margin-top: var(--termi-spacing-sm, 8px);
    background-color: transparent;
    color: var(--termi-text-secondary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-md, 6px);
    font-weight: 500;
    transition: background-color 0.15s, color 0.15s;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--termi-spacing-xs, 6px);
    justify-content: center;
  }

  .settings-btn:hover {
    background-color: var(--termi-bg-surface);
    color: var(--termi-text-primary);
  }
</style>
