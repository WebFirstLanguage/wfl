/** Recognize the version output of a WFL language server executable. */
export function isWflLspVersion(stdout: string): boolean {
  return stdout.trim().startsWith('wfl-lsp version');
}
