/**
 * Canonical flag-catalog contract.
 *
 * This TypeSpec file and flagcatalog.json are independent, human-maintained
 * peer authorities. Generated JSON Schema and language projections are
 * comparison evidence only; neither authored authority has precedence.
 */
import "@typespec/json-schema";

namespace Flags2Env.Contracts.V1;

@TypeSpec.JsonSchema.extension("additionalProperties", false)
model FlagCatalog {
  @minLength(1)
  id: string;

  @minLength(1)
  revision: string;

  payload: Record<unknown>;
}
