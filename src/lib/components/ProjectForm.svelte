<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { addProject, updateProject } from '$lib/stores/projects.svelte';
  import type { Project } from '$lib/types';
  import { i18n } from '$lib/i18n/index.svelte';

  const t = $derived(i18n.t);

  let {
    editProject,
    onClose,
  }: {
    editProject?: Project;
    onClose: () => void;
  } = $props();

  let name = $state(editProject?.name ?? '');
  let path = $state(editProject?.path ?? '');
  let error = $state('');
  let loading = $state(false);

  const isEditMode = $derived(!!editProject);

  async function selectFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t.projectForm.browseTitle,
      });
      if (selected) {
        if (typeof selected === 'string') {
          path = selected;
        }
      }
    } catch (e) {
      console.error('Folder selection failed:', e);
      error = t.projectForm.folderSelectFailed + String(e);
    }
  }

  async function handleSubmit() {
    error = '';

    if (!name.trim()) {
      error = t.projectForm.nameRequired;
      return;
    }
    if (!path) {
      error = t.projectForm.pathRequired;
      return;
    }

    loading = true;
    try {
      if (isEditMode && editProject) {
        await updateProject(editProject.id, name.trim(), path);
      } else {
        await addProject(name.trim(), path);
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="project-form">
  <h3>{isEditMode ? t.projectForm.editTitle : t.projectForm.addTitle}</h3>

  <div class="form-group">
    <label for="project-name">{t.projectForm.nameLabel}</label>
    <input
      id="project-name"
      type="text"
      bind:value={name}
      placeholder={t.projectForm.namePlaceholder}
      disabled={loading}
    />
  </div>

  <div class="form-group">
    <label for="project-path">{t.projectForm.pathLabel}</label>
    <div class="path-input">
      <input
        id="project-path"
        type="text"
        value={path}
        placeholder={t.projectForm.pathPlaceholder}
        readonly
      />
      <button class="browse-btn" onclick={selectFolder} disabled={loading}>
        {t.projectForm.browse}
      </button>
    </div>
  </div>

  {#if error}
    <p class="error-message">{error}</p>
  {/if}

  <div class="form-actions">
    <button class="cancel-btn" onclick={onClose} disabled={loading}>
      {t.projectForm.cancel}
    </button>
    <button class="submit-btn" onclick={handleSubmit} disabled={loading}>
      {#if loading}
        {isEditMode ? t.projectForm.editing : t.projectForm.adding}
      {:else}
        {isEditMode ? t.projectForm.edit : t.projectForm.add}
      {/if}
    </button>
  </div>
</div>

<style>
  .project-form {
    padding: var(--termi-spacing-md, 16px);
    border-bottom: 1px solid var(--termi-border);
    background-color: var(--termi-bg-surface);
  }

  .project-form h3 {
    font-size: var(--termi-font-size-base, 15px);
    font-weight: 600;
    margin-bottom: var(--termi-spacing-sm, 8px);
    color: var(--termi-text-primary);
  }

  .form-group {
    margin-bottom: var(--termi-spacing-sm, 8px);
  }

  .form-group label {
    display: block;
    font-size: var(--termi-font-size-sm, 13px);
    color: var(--termi-text-secondary);
    margin-bottom: var(--termi-spacing-xs, 4px);
  }

  .form-group input {
    width: 100%;
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-sm, 8px);
    background-color: var(--termi-bg-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-md, 14px);
  }

  .form-group input:focus-visible {
    border-color: var(--termi-accent);
    outline: 2px solid var(--termi-accent);
    outline-offset: -2px;
  }

  .path-input {
    display: flex;
    gap: var(--termi-spacing-xs, 6px);
  }

  .path-input input {
    flex: 1;
  }

  .browse-btn {
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-sm, 10px);
    background-color: var(--termi-bg-primary);
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-primary);
    font-size: var(--termi-font-size-sm, 13px);
    white-space: nowrap;
  }

  .browse-btn:hover {
    background-color: var(--termi-bg-surface);
  }

  .error-message {
    font-size: var(--termi-font-size-sm, 13px);
    margin-bottom: var(--termi-spacing-sm, 8px);
  }

  .form-actions {
    display: flex;
    gap: var(--termi-spacing-sm, 8px);
    justify-content: flex-end;
  }

  .cancel-btn {
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-md, 14px);
    background: none;
    border: 1px solid var(--termi-border);
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-md, 14px);
  }

  .cancel-btn:hover {
    background-color: var(--termi-bg-primary);
  }

  .submit-btn {
    padding: var(--termi-spacing-xs, 6px) var(--termi-spacing-md, 14px);
    background-color: var(--termi-accent);
    border: none;
    border-radius: var(--termi-radius-sm, 4px);
    color: var(--termi-bg-primary);
    font-weight: 500;
    font-size: var(--termi-font-size-md, 14px);
  }

  .submit-btn:hover {
    background-color: var(--termi-accent-hover);
  }

  .submit-btn:disabled,
  .cancel-btn:disabled,
  .browse-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
