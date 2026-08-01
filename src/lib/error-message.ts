/**
 * What to show the user for a value that was thrown or rejected. A Tauri
 * command rejects with a plain string, application code throws an `Error`, and
 * anything else has to be stringified rather than rendered as [object Object].
 */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
