import { invoke as tauriInvoke } from '@tauri-apps/api/core';

/**
 * Wrapper around Tauri's `invoke` that calls a Rust command by name.
 * @param cmd - The Rust command identifier (e.g. 'get_projects', 'launch_terminal').
 * @param args - Optional key-value arguments forwarded to the Rust handler.
 * @returns A promise resolving to the value returned by the Rust command, typed as `T`.
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}
