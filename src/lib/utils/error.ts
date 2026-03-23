/**
 * Shared error handling utilities for store files.
 *
 * Extracts the duplicated error-message extraction and try/catch boilerplate
 * into reusable functions so every store handles errors consistently.
 */

/**
 * Extracts a human-readable message from an unknown thrown value.
 *
 * Replaces the repeated pattern:
 *   `e instanceof Error ? e.message : String(e)`
 */
export function extractErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Wraps an async operation with standardised error handling:
 *   1. Clears the current error state (`setError(null)`).
 *   2. Executes `fn`.
 *   3. On failure, sets the error message and re-throws.
 *
 * The behaviour is identical to the try/catch blocks previously duplicated
 * across every store function.
 */
export async function withErrorHandling<T>(
  setError: (msg: string | null) => void,
  fn: () => Promise<T>,
): Promise<T> {
  try {
    setError(null);
    return await fn();
  } catch (e) {
    setError(extractErrorMessage(e));
    throw e;
  }
}

/**
 * UI-level error wrapper that does NOT re-throw.
 *
 * Suitable for component event handlers where errors should be captured
 * into reactive state and displayed to the user, not propagated further.
 *
 *   1. Clears the current error state (`setError(null)`).
 *   2. Executes `fn`.
 *   3. On failure, sets the error message (does **not** re-throw).
 */
export async function catchError<T = void>(
  setError: (msg: string | null) => void,
  fn: () => Promise<T>,
): Promise<void> {
  try {
    setError(null);
    await fn();
  } catch (e) {
    setError(extractErrorMessage(e));
  }
}
