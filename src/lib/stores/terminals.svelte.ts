import { listen } from '@tauri-apps/api/event';
import { invoke } from '$lib/api/invoke';
import type { TerminalInstance, StatusChangedPayload, ExternalTerminalInfo } from '$lib/types';
import { extractErrorMessage, withErrorHandling } from '$lib/utils/error';
import {
  ALL_PROJECTS_ID,
  EVENT_TERMINAL_CLOSED,
  EVENT_TERMINAL_STATUS_CHANGED,
  EVENT_NOTIFICATION_CLICKED,
  EVENT_TERMINAL_AUTO_ATTACHED,
  EVENT_TERMINAL_ORDER_CHANGED,
} from '$lib/constants';
let terminals = $state.raw<TerminalInstance[]>([]);
let activeTabId = $state<string | null>(null);
let activeProjectId = $state<string | null>(null);
let lastError = $state<string | null>(null);
let tabOrder = $state<string[]>([]);
let launching = $state(false);

const _activeTerminal = $derived(terminals.find(t => t.id === activeTabId) ?? null);
const _hasTerminals = $derived(terminals.length > 0);
const _activeProjectTerminals = $derived(
  activeProjectId === ALL_PROJECTS_ID
    ? terminals
    : activeProjectId
      ? terminals.filter(t => t.projectId === activeProjectId)
      : []
);
/** terminals 배열에서 프로젝트별 터미널 수를 집계하여 반환. launchedAt 기준 안정 정렬. */
const _projectsWithTerminals = $derived(
  (() => {
    const map = new Map<string, { projectId: string; projectName: string; terminalCount: number }>();
    const firstLaunch = new Map<string, string>();
    for (const t of terminals) {
      const existing = map.get(t.projectId);
      if (existing) {
        existing.terminalCount++;
      } else {
        map.set(t.projectId, { projectId: t.projectId, projectName: t.projectName, terminalCount: 1 });
      }
      // 프로젝트별 가장 이른 launchedAt 추적 (안정 정렬용)
      const prev = firstLaunch.get(t.projectId);
      if (!prev || t.launchedAt < prev) {
        firstLaunch.set(t.projectId, t.launchedAt);
      }
    }
    return Array.from(map.values()).sort((a, b) =>
      (firstLaunch.get(a.projectId) ?? '').localeCompare(firstLaunch.get(b.projectId) ?? '')
    );
  })()
);

/** tabOrder 기준으로 projectsWithTerminals를 정렬하여 반환. tabOrder에 없는 프로젝트는 뒤에 추가된다. */
const _orderedProjectsWithTerminals = $derived(
  (() => {
    if (tabOrder.length === 0) return _projectsWithTerminals;
    const orderMap = new Map(tabOrder.map((id, idx) => [id, idx]));
    return [..._projectsWithTerminals].sort((a, b) => {
      const aIdx = orderMap.get(a.projectId) ?? Number.MAX_SAFE_INTEGER;
      const bIdx = orderMap.get(b.projectId) ?? Number.MAX_SAFE_INTEGER;
      return aIdx - bIdx;
    });
  })()
);

export const terminalsStore = {
  get terminals() { return terminals; },
  get activeTabId() { return activeTabId; },
  set activeTabId(id: string | null) { activeTabId = id; },
  get activeProjectId() { return activeProjectId; },
  set activeProjectId(id: string | null) { activeProjectId = id; },
  get activeTerminal() { return _activeTerminal; },
  get hasTerminals() { return _hasTerminals; },
  get lastError() { return lastError; },
  clearError() { lastError = null; },
  /** 현재 활성 프로젝트에 속한 터미널 목록을 반환한다. activeProjectId와 terminals에 반응한다. ALL_PROJECTS_ID이면 전체 반환. */
  get activeProjectTerminals() { return _activeProjectTerminals; },
  get projectsWithTerminals(): Array<{ projectId: string; projectName: string; terminalCount: number }> {
    return _projectsWithTerminals;
  },
  /** 탭 순서 배열. 드래그 앤 드롭으로 변경된 순서를 저장한다. */
  get tabOrder() { return tabOrder; },
  set tabOrder(order: string[]) { tabOrder = order; },
  /** tabOrder 기준으로 정렬된 projectsWithTerminals를 반환한다. */
  get orderedProjectsWithTerminals(): Array<{ projectId: string; projectName: string; terminalCount: number }> {
    return _orderedProjectsWithTerminals;
  },
  /** 터미널 생성 중 여부. UI에서 중복 클릭 방지에 사용. */
  get launching() { return launching; },
};

/**
 * 새 터미널 인스턴스를 생성하고 활성 탭으로 설정한다.
 */
export async function launchTerminal(projectId: string): Promise<TerminalInstance> {
  if (launching) return undefined as unknown as TerminalInstance;
  launching = true;
  try {
    return await withErrorHandling((msg) => { lastError = msg; }, async () => {
      const terminal = await invoke<TerminalInstance>('launch_terminal', { projectId });
      terminals = [...terminals, terminal];
      activeProjectId = terminal.projectId;
      activeTabId = terminal.id;
      return terminal;
    });
  } finally {
    launching = false;
  }
}

/**
 * 지정된 터미널의 Windows Terminal 창을 포그라운드로 이동한다.
 * Calls: `focus_terminal`
 */
export async function focusTerminal(terminalId: string): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    await invoke<void>('focus_terminal', { terminalId });
    activeTabId = terminalId;

    const terminal = terminals.find(t => t.id === terminalId);
    if (terminal && activeProjectId !== ALL_PROJECTS_ID) {
      activeProjectId = terminal.projectId;
    }
  });
}

/**
 * 지정된 터미널을 닫고 목록에서 제거한다.
 * Calls: `close_terminal`
 */
export async function closeTerminal(terminalId: string): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    await invoke<void>('close_terminal', { terminalId });
  });
}

/** 지정된 터미널의 이름을 변경한다. Calls: `rename_terminal` */
export async function renameTerminal(terminalId: string, newName: string): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    await invoke<void>('rename_terminal', { terminalId, newName });
    terminals = terminals.map(t =>
      t.id === terminalId ? { ...t, terminalName: newName } : t
    );
  });
}

/** 지정된 터미널의 알림 설정을 토글한다. Calls: `toggle_terminal_notification` */
export async function toggleNotification(terminalId: string): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    await invoke<void>('toggle_terminal_notification', { terminalId });
    terminals = terminals.map(t =>
      t.id === terminalId ? { ...t, notificationEnabled: !t.notificationEnabled } : t
    );
  });
}

export function setActiveTab(terminalId: string): void {
  activeTabId = terminalId;
}

export function getTerminalByProjectId(projectId: string): TerminalInstance | undefined {
  return terminals.find(t => t.projectId === projectId && t.status === 'running');
}

/**
 * 외부 Windows Terminal 윈도우를 탐색한다.
 * Calls: `discover_external_terminals`
 */
export async function discoverExternalTerminals(): Promise<ExternalTerminalInfo[]> {
  return withErrorHandling((msg) => { lastError = msg; }, async () => {
    return await invoke<ExternalTerminalInfo[]>('discover_external_terminals');
  });
}

/**
 * 외부 터미널 탭을 프로젝트에 어태치한다.
 * Calls: `attach_terminal`
 */
export async function attachTerminal(
  hwnd: number,
  runtimeId: number[],
  tabTitle: string,
  projectId: string,
): Promise<TerminalInstance> {
  return withErrorHandling((msg) => { lastError = msg; }, async () => {
    const terminal = await invoke<TerminalInstance>('attach_terminal', {
      hwnd, runtimeId, tabTitle, projectId,
    });
    terminals = [...terminals, terminal];
    activeProjectId = terminal.projectId;
    activeTabId = terminal.id;
    return terminal;
  });
}

/** 터미널 관련 Tauri 이벤트 리스너를 초기화하고, cleanup 함수를 반환한다. */
export async function initTerminalEvents(): Promise<() => void> {
  const unlistenClosed = await listen<string>(EVENT_TERMINAL_CLOSED, (event) => {
    const closedId = event.payload;
    terminals = terminals.filter(t => t.id !== closedId);

    if (activeTabId === closedId) {
      const sameProjectTerminal = activeProjectId
        ? terminals.find(t => t.projectId === activeProjectId)
        : null;
      activeTabId = sameProjectTerminal?.id
        ?? (terminals.length > 0 ? terminals[terminals.length - 1].id : null);
    }

    if (activeProjectId && !terminals.some(t => t.projectId === activeProjectId)) {
      activeProjectId = terminals.length > 0
        ? terminals[terminals.length - 1].projectId
        : null;
    }
  });

  const unlistenStatus = await listen<StatusChangedPayload>(EVENT_TERMINAL_STATUS_CHANGED, (event) => {
    const { terminalId, status, monitored, lastIdleAt } = event.payload;

    // terminalId로 직접 매칭
    const matchedIndex = terminals.findIndex(t => t.id === terminalId);

    const updates: Partial<TerminalInstance> = { activity: status, monitored };
    if (lastIdleAt) updates.lastIdleAt = lastIdleAt;

    if (matchedIndex === -1) {
      // fallback: projectPath로 매칭
      const { projectPath } = event.payload;
      const normalizedPath = projectPath.replace(/[\\/]+$/, '').toLowerCase();
      const pathIndex = terminals.findIndex(
        t => t.projectPath.replace(/[\\/]+$/, '').toLowerCase() === normalizedPath
      );
      if (pathIndex !== -1) {
        terminals = terminals.map((t, i) =>
          i === pathIndex ? { ...t, ...updates } : t
        );
      }
      return;
    }

    terminals = terminals.map((t, i) =>
      i === matchedIndex ? { ...t, ...updates } : t
    );
  });

  // 알림 클릭 이벤트: Rust에서 tauri-winrt-notification의 on_activated 콜백이
  // notification-clicked 이벤트를 emit하면 여기서 처리
  const unlistenNotifClick = await listen<string>(EVENT_NOTIFICATION_CLICKED, (event) => {
    const terminalId = event.payload;
    if (terminalId) {
      invoke<void>('acknowledge_notification', { terminalId }).catch((e) => {
        lastError = extractErrorMessage(e);
      });
      focusTerminal(terminalId).catch((e) => { lastError = extractErrorMessage(e); });
    }
  });

  const unlistenAutoAttached = await listen<TerminalInstance[]>(EVENT_TERMINAL_AUTO_ATTACHED, (event) => {
    const attached = event.payload;
    if (attached && attached.length > 0) {
      terminals = [...terminals, ...attached];
    }
  });

  const unlistenOrder = await listen<string[]>(EVENT_TERMINAL_ORDER_CHANGED, (event) => {
    const orderedIds = event.payload;
    if (orderedIds.length < 2) return;

    const orderMap = new Map(orderedIds.map((id, idx) => [id, idx]));
    terminals = [...terminals].sort((a, b) => {
      const aIdx = orderMap.get(a.id);
      const bIdx = orderMap.get(b.id);
      if (aIdx !== undefined && bIdx !== undefined) return aIdx - bIdx;
      if (aIdx !== undefined) return -1;
      if (bIdx !== undefined) return 1;
      return 0;
    });
  });

  return () => {
    unlistenClosed();
    unlistenStatus();
    unlistenNotifClick();
    unlistenAutoAttached();
    unlistenOrder();
  };
}
