// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';

// Mock Tauri invoke
vi.mock('$lib/api/invoke', () => ({
  invoke: vi.fn(),
}));

// Mock @tauri-apps/api/event — listen captures callbacks for later invocation
const listenCallbacks = new Map<string, (event: { payload: unknown }) => void>();
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (eventName: string, callback: (event: { payload: unknown }) => void) => {
    listenCallbacks.set(eventName, callback);
    return vi.fn(); // unlisten function
  }),
}));

import { EVENT_TERMINAL_ORDER_CHANGED } from '$lib/constants';
import type { TerminalInstance } from '$lib/types';
import { terminalsStore, initTerminalEvents } from './terminals.svelte';

function makeTerminal(overrides: Partial<TerminalInstance> & { id: string; projectId: string }): TerminalInstance {
  return {
    projectName: 'Test',
    projectPath: '/test',
    terminalName: 'Terminal',
    status: 'running',
    launchedAt: '2026-01-01',
    activity: 'active',
    notificationEnabled: false,
    monitored: false,
    attached: false,
    ...overrides,
  };
}

/** Helper: emit a synthetic Tauri event by event name */
function emitEvent(eventName: string, payload: unknown) {
  const cb = listenCallbacks.get(eventName);
  if (!cb) throw new Error(`No listener registered for event: ${eventName}`);
  cb({ payload });
}

describe('EVENT_TERMINAL_ORDER_CHANGED constant', () => {
  it('should be exported with value "terminal-order-changed"', () => {
    expect(EVENT_TERMINAL_ORDER_CHANGED).toBe('terminal-order-changed');
  });
});

describe('terminal-order-changed event handler', () => {
  let cleanup: () => void;

  beforeEach(async () => {
    // Clean up existing terminals via terminal-closed events
    for (const t of terminalsStore.terminals) {
      const closeCb = listenCallbacks.get('terminal-closed');
      if (closeCb) closeCb({ payload: t.id });
    }

    vi.clearAllMocks();
    listenCallbacks.clear();

    // Init to register listeners
    cleanup = await initTerminalEvents();
  });

  it('reorders terminals array when event is received with valid ordered IDs', () => {
    // Setup: insert terminals via auto-attach event
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-a' });
    const t3 = makeTerminal({ id: 'term-3', projectId: 'proj-b' });
    emitEvent('terminal-auto-attached', [t1, t2, t3]);

    // Verify initial order
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-1', 'term-2', 'term-3']);

    // Act: emit terminal-order-changed with reversed order
    emitEvent('terminal-order-changed', ['term-3', 'term-1', 'term-2']);

    // Assert: terminals should be reordered
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-3', 'term-1', 'term-2']);
  });

  it('does not reorder when payload has fewer than 2 IDs', () => {
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-a' });
    emitEvent('terminal-auto-attached', [t1, t2]);

    // Act: emit with only 1 ID
    emitEvent('terminal-order-changed', ['term-1']);

    // Assert: order unchanged
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-1', 'term-2']);
  });

  it('does not reorder when payload is empty', () => {
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-a' });
    emitEvent('terminal-auto-attached', [t1, t2]);

    emitEvent('terminal-order-changed', []);

    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-1', 'term-2']);
  });

  it('preserves terminals not in the ordered IDs at the end', () => {
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-a' });
    const t3 = makeTerminal({ id: 'term-3', projectId: 'proj-b' });
    emitEvent('terminal-auto-attached', [t1, t2, t3]);

    // Only reorder term-2 and term-1, term-3 not mentioned
    emitEvent('terminal-order-changed', ['term-2', 'term-1']);

    // term-2 and term-1 should come first (in that order), term-3 at the end
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-2', 'term-1', 'term-3']);
  });

  it('ignores IDs not present in terminals array', () => {
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-a' });
    emitEvent('terminal-auto-attached', [t1, t2]);

    // payload contains unknown ID
    emitEvent('terminal-order-changed', ['term-2', 'unknown-id', 'term-1']);

    // Should still reorder known IDs correctly
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-2', 'term-1']);
  });

  it('works independently from tabOrder (project order)', () => {
    const t1 = makeTerminal({ id: 'term-1', projectId: 'proj-a' });
    const t2 = makeTerminal({ id: 'term-2', projectId: 'proj-b' });
    emitEvent('terminal-auto-attached', [t1, t2]);

    // Set tabOrder for projects
    terminalsStore.tabOrder = ['proj-b', 'proj-a'];

    // Now reorder terminals independently
    emitEvent('terminal-order-changed', ['term-2', 'term-1']);

    // Terminals should be reordered per the event
    expect(terminalsStore.terminals.map(t => t.id)).toEqual(['term-2', 'term-1']);
    // tabOrder should remain unchanged
    expect(terminalsStore.tabOrder).toEqual(['proj-b', 'proj-a']);
  });

  it('cleanup unregisters the order-changed listener', async () => {
    // The listen mock returns unlisten functions (vi.fn())
    // After cleanup(), those unlisten functions should have been called
    const { listen: listenMock } = await import('@tauri-apps/api/event');
    const mockListen = listenMock as Mock; // TYPE-ASSERT: vi.mock wraps function but TS infers original type

    // Get all unlisten functions returned by listen calls
    const unlistenPromises = mockListen.mock.results.map((r: { type: string; value: unknown }) => r.value);
    const unlistenFns = await Promise.all(unlistenPromises);

    // Before cleanup, none should be called
    for (const fn of unlistenFns) {
      (fn as Mock).mockClear(); // TYPE-ASSERT: vi.fn() returns Mock but Promise.all loses type info
    }

    cleanup();

    // After cleanup, all unlisten functions should have been called once
    for (const fn of unlistenFns) {
      expect(fn).toHaveBeenCalledOnce();
    }
  });
});
