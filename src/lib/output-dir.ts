import { downloadDir } from "@tauri-apps/api/path";

const OUTPUT_DIR_KEY = "pdf-tools.output-dir";

export function rememberOutputDir(dir: string): void {
  try {
    localStorage.setItem(OUTPUT_DIR_KEY, dir);
  } catch {
    // Storage may be unavailable without preventing a merge.
  }
}

export async function defaultOutputDir(): Promise<string> {
  try {
    const remembered = localStorage.getItem(OUTPUT_DIR_KEY);
    if (remembered) {
      return remembered;
    }
  } catch {
    // Fall back to the operating system default when storage is unavailable.
  }

  return downloadDir();
}

export function parentDir(filePath: string): string {
  const separatorIndex = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
  return separatorIndex < 0 ? "" : filePath.slice(0, separatorIndex);
}

export function joinPath(dir: string, name: string): string {
  if (!dir) {
    return name;
  }

  const separator = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  return dir.endsWith(separator) ? `${dir}${name}` : `${dir}${separator}${name}`;
}
