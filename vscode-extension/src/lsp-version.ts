import { valid } from 'semver';

/** Recognize the version output of a WFL language server executable. */
export function isWflLspVersion(stdout: string): boolean {
  const match = /^wfl-lsp (?:version )?(\S+)$/.exec(stdout.trim());
  return match !== null && valid(match[1]) !== null;
}
