import { invoke } from '$lib/api/invoke';
import type { Project } from '$lib/types';
import { extractErrorMessage } from '$lib/utils/error';

let projects = $state.raw<Project[]>([]);
let error = $state<string | null>(null);

export const projectsStore = {
  get projects() { return projects; },
  get error() { return error; },
};

/** Rust `get_projects` 커맨드를 호출하여 프로젝트 목록을 로드한다. sortOrder 기준 정렬 적용. */
export async function loadProjects() {
  try {
    error = null;
    const loaded = await invoke<Project[]>('get_projects');
    projects = loaded.sort((a, b) => a.sortOrder - b.sortOrder);
  } catch (e) {
    error = extractErrorMessage(e);
    console.error('프로젝트 목록 로드 실패:', e);
    throw e;
  }
}

/** Rust `add_project` 커맨드를 호출하여 새 프로젝트를 추가한다. */
export async function addProject(name: string, path: string): Promise<Project> {
  try {
    const project = await invoke<Project>('add_project', { name, path });
    projects = [...projects, project];
    return project;
  } catch (e) {
    error = extractErrorMessage(e);
    throw e;
  }
}

/** Rust `update_project` 커맨드를 호출하여 프로젝트 정보를 수정한다. */
export async function updateProject(id: string, name: string, path: string): Promise<Project> {
  try {
    const updated = await invoke<Project>('update_project', { id, name, path });
    projects = projects.map((p) => (p.id === id ? updated : p));
    return updated;
  } catch (e) {
    error = extractErrorMessage(e);
    throw e;
  }
}

/** Rust `reorder_projects` 커맨드를 호출하여 프로젝트 순서를 변경한다. */
export async function reorderProjects(orderedIds: string[]) {
  try {
    const reordered = await invoke<Project[]>('reorder_projects', { projectIds: orderedIds });
    projects = reordered;
  } catch (e) {
    error = extractErrorMessage(e);
    throw e;
  }
}

/** Rust `delete_project` 커맨드를 호출하여 프로젝트를 삭제한다. */
export async function deleteProject(id: string): Promise<void> {
  try {
    await invoke<void>('delete_project', { id });
    projects = projects.filter((p) => p.id !== id);
  } catch (e) {
    error = extractErrorMessage(e);
    throw e;
  }
}
