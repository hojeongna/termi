import { invoke } from '$lib/api/invoke';
import type { Settings } from '$lib/types';
import { withErrorHandling } from '$lib/utils/error';

let settings = $state<Settings | null>(null);
let lastError = $state<string | null>(null);

export const settingsStore = {
  get settings() { return settings; },
  get lastError() { return lastError; },
};

/** Rust `get_settings` 커맨드를 호출하여 사용자 설정을 로드한다. */
export async function loadSettings(): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    settings = await invoke<Settings>('get_settings');
  });
}

/** Rust `update_settings` 커맨드를 호출하여 사용자 설정을 저장한다. */
export async function updateSettings(newSettings: Settings): Promise<void> {
  await withErrorHandling((msg) => { lastError = msg; }, async () => {
    await invoke<void>('update_settings', { settings: newSettings });
    settings = newSettings;
  });
}

/** 현재 설정에 부분 변경을 병합하여 저장한다. 현재 설정이 없으면 무시한다. */
export async function savePartialSettings(partial: Partial<Settings>): Promise<void> {
  const current = settings;
  if (!current) return;
  await updateSettings({ ...current, ...partial });
}
