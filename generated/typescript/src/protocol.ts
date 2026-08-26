import { InterfaceError } from "./errors";
import type { FlagCatalog } from "./types";

export function parseFlagCatalog(
  id: string,
  revision: string,
  payload: Record<string, unknown>,
): FlagCatalog {
  if (!id.trim()) {
    throw new InterfaceError("empty_id");
  }
  if (!revision.trim()) {
    throw new InterfaceError("empty_revision");
  }
  return { id, revision, payload };
}

