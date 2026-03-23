// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock Tauri invoke
vi.mock('$lib/api/invoke', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '$lib/api/invoke';
import { projectsStore, loadProjects, reorderProjects } from './projects.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('loadProjects', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('sorts projects by sortOrder after loading', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { id: 'c', name: 'C', path: '/c', createdAt: '2026-01-03', sortOrder: 2 },
      { id: 'a', name: 'A', path: '/a', createdAt: '2026-01-01', sortOrder: 0 },
      { id: 'b', name: 'B', path: '/b', createdAt: '2026-01-02', sortOrder: 1 },
    ]);

    await loadProjects();

    expect(mockedInvoke).toHaveBeenCalledWith('get_projects');
    expect(projectsStore.projects[0].id).toBe('a');
    expect(projectsStore.projects[1].id).toBe('b');
    expect(projectsStore.projects[2].id).toBe('c');
  });

  it('sets error state when invoke fails', async () => {
    mockedInvoke.mockRejectedValueOnce('Network error');
    await expect(loadProjects()).rejects.toThrow();
    expect(projectsStore.error).toBeTruthy();
  });
});

describe('reorderProjects', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('calls invoke with reorder_projects and updates local state', async () => {
    // First load projects
    mockedInvoke.mockResolvedValueOnce([
      { id: 'a', name: 'A', path: '/a', createdAt: '2026-01-01', sortOrder: 0 },
      { id: 'b', name: 'B', path: '/b', createdAt: '2026-01-02', sortOrder: 1 },
    ]);
    await loadProjects();

    // Mock reorder response
    mockedInvoke.mockResolvedValueOnce([
      { id: 'b', name: 'B', path: '/b', createdAt: '2026-01-02', sortOrder: 0 },
      { id: 'a', name: 'A', path: '/a', createdAt: '2026-01-01', sortOrder: 1 },
    ]);

    await reorderProjects(['b', 'a']);

    expect(mockedInvoke).toHaveBeenCalledWith('reorder_projects', { projectIds: ['b', 'a'] });
    expect(projectsStore.projects[0].id).toBe('b');
    expect(projectsStore.projects[1].id).toBe('a');
  });

  it('sets error state and re-throws when invoke fails', async () => {
    mockedInvoke.mockRejectedValueOnce('Network error');
    await expect(reorderProjects(['a'])).rejects.toThrow();
    expect(projectsStore.error).toBeTruthy();
  });
});
