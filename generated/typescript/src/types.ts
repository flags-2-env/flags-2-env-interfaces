export const PROTOCOL_VERSION = "1" as const;
export const SCHEMA_REVISION = "flags-2-env-0001" as const;

export interface Health {
  ok: boolean;
  service: string;
  protocol: string;
}

export interface FlagCatalog {
  id: string;
  revision: string;
  payload: Record<string, unknown>;
}

