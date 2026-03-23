import { invoke } from '$lib/api/invoke';
import type { ThemeFile, ThemeListEntry, ThemeType } from '$lib/types';
import { extractErrorMessage } from '$lib/utils/error';
import { DEFAULT_DARK_THEME_ID } from '$lib/constants';

export const DEFAULT_DARK: Record<string, string> = {
  'bg-primary': '#1e1e2e',
  'bg-secondary': '#181825',
  'bg-surface': '#313244',
  'text-primary': '#cdd6f4',
  'text-secondary': '#a6adc8',
  'accent': '#89b4fa',
  'accent-hover': '#74c7ec',
  'border': '#45475a',
  'danger': '#f38ba8',
  'success': '#a6e3a1',
  'status-working': '#22c55e',
  'status-waiting': '#eab308',
  'status-completed': '#ef4444',
};

export const DEFAULT_LIGHT: Record<string, string> = {
  'bg-primary': '#eff1f5',
  'bg-secondary': '#e6e9ef',
  'bg-surface': '#ccd0da',
  'text-primary': '#4c4f69',
  'text-secondary': '#6c6f85',
  'accent': '#1e66f5',
  'accent-hover': '#209fb5',
  'border': '#bcc0cc',
  'danger': '#d20f39',
  'success': '#40a02b',
  'status-working': '#40a02b',
  'status-waiting': '#df8e1d',
  'status-completed': '#d20f39',
};

/** 테마 타입에 맞는 기본 색상과 커스텀 색상을 병합한다. */
export function mergeThemeColors(
  themeType: ThemeType,
  colors: Record<string, string>,
): Record<string, string> {
  const base = themeType === 'dark' ? DEFAULT_DARK : DEFAULT_LIGHT;
  return { ...base, ...colors };
}

/** 병합된 색상을 document.documentElement의 CSS 변수로 적용한다. */
export function applyThemeToDocument(colors: Record<string, string>): void {
  for (const [key, value] of Object.entries(colors)) {
    document.documentElement.style.setProperty(`--termi-${key}`, value);
  }
}

let currentThemeId = $state<string>(DEFAULT_DARK_THEME_ID);
let availableThemes = $state.raw<ThemeListEntry[]>([]);
let lastError = $state<string | null>(null);

/**
 * Read-only reactive store for theme state.
 * State mutations should go through the exported async functions
 * (loadAvailableThemes, applyTheme, saveCustomTheme, deleteCustomTheme, initializeTheme).
 */
export const themeStore = {
  get currentThemeId() { return currentThemeId; },
  get availableThemes() { return availableThemes; },
  get lastError() { return lastError; },
};

/** lastError 상태를 초기화한다. */
export function clearError() { lastError = null; }

/**
 * 사용 가능한 테마 목록을 Rust에서 로드하여 `availableThemes` 상태에 저장한다.
 *
 * Calls: `get_available_themes`
 * @returns 반환값 없음. 성공 시 `themeStore.availableThemes`가 갱신된다.
 * @throws Rust 커맨드 실패 시 에러를 상위로 re-throw한다.
 */
export async function loadAvailableThemes(): Promise<void> {
  try {
    availableThemes = await invoke<ThemeListEntry[]>('get_available_themes');
  } catch (e) {
    lastError = extractErrorMessage(e);
    console.error('Failed to load available themes:', e);
    throw e;
  }
}

/**
 * 지정한 테마 ID의 테마를 로드하고 문서에 CSS 변수로 적용한다.
 *
 * Calls: `get_theme`
 * @returns 반환값 없음. 성공 시 `themeStore.currentThemeId`가 갱신된다.
 * @throws Rust 커맨드 실패 시 에러를 상위로 re-throw한다.
 */
export async function applyTheme(themeId: string): Promise<void> {
  try {
    const theme = await invoke<ThemeFile>('get_theme', { themeId });
    const merged = mergeThemeColors(theme.type, theme.colors);
    applyThemeToDocument(merged);
    currentThemeId = themeId;
  } catch (e) {
    lastError = extractErrorMessage(e);
    console.error('Failed to apply theme:', e);
    throw e;
  }
}

/**
 * 저장된 테마 ID로 초기 테마를 로드한다. 실패 시 기본 다크 테마를 적용하고 에러를 삼킨다.
 *
 * Calls: `get_theme` (via `applyTheme`)
 * @returns 반환값 없음. 성공/실패 모두 테마가 적용된 상태로 종료된다.
 * @throws 에러를 re-throw하지 않는다. 실패 시 `themeStore.lastError`에 기록된다.
 */
export async function initializeTheme(savedThemeId: string): Promise<void> {
  try {
    await applyTheme(savedThemeId);
  } catch (e) {
    lastError = extractErrorMessage(e);
    console.error('Failed to initialize theme:', e);
    applyThemeToDocument(DEFAULT_DARK);
    currentThemeId = DEFAULT_DARK_THEME_ID;
  }
}

/**
 * 커스텀 테마를 파일로 저장하고 테마 목록을 갱신한다.
 *
 * Calls: `save_theme`, `get_available_themes` (via `loadAvailableThemes`)
 * @returns 반환값 없음. 성공 시 `themeStore.availableThemes`가 갱신된다.
 * @throws Rust 커맨드 실패 시 에러를 상위로 re-throw한다.
 */
export async function saveCustomTheme(themeId: string, theme: ThemeFile): Promise<void> {
  try {
    await invoke<void>('save_theme', { themeId, theme });
    await loadAvailableThemes();
  } catch (e) {
    lastError = extractErrorMessage(e);
    console.error('Failed to save custom theme:', e);
    throw e;
  }
}

/**
 * 커스텀 테마를 삭제하고 목록을 갱신한다. 삭제된 테마가 현재 적용 중이면 기본 다크 테마로 전환한다.
 *
 * Calls: `delete_theme`, `get_available_themes` (via `loadAvailableThemes`), `get_theme` (via `applyTheme`)
 * @returns 반환값 없음. 성공 시 `themeStore.availableThemes`가 갱신된다.
 * @throws Rust 커맨드 실패 시 에러를 상위로 re-throw한다.
 */
export async function deleteCustomTheme(themeId: string): Promise<void> {
  try {
    await invoke<void>('delete_theme', { themeId });
    await loadAvailableThemes();
    if (currentThemeId === themeId) {
      await applyTheme(DEFAULT_DARK_THEME_ID);
    }
  } catch (e) {
    lastError = extractErrorMessage(e);
    console.error('Failed to delete custom theme:', e);
    throw e;
  }
}

/** 현재 테마의 JSON 템플릿을 생성한다. */
export function generateThemeTemplate(baseType: ThemeType): string {
  const base = baseType === 'dark' ? DEFAULT_DARK : DEFAULT_LIGHT;
  const template = {
    name: 'My Custom Theme',
    description: '',
    type: baseType,
    colors: { ...base },
  };
  return JSON.stringify(template, null, 2);
}
