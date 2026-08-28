/**
 * Runtime contract enforcement against JSON Schema.
 *
 * Port of runtime/rust/contract_guard.rs — same posture, same vocabulary, so a
 * violation reported by a Rust service and one reported by a TypeScript client
 * land in ores-otel with identical field names.
 *
 * JSON Schema is a cross-check here, not the source of the generated types.
 * When the schema and the generated types disagree, one of them has drifted.
 *
 *   dev / test / e2e  validate everything, reject on violation
 *   production        validate a sample, report, never reject
 */

import type { ValidateFunction } from 'ajv';

export type Direction = 'request' | 'response';

export const enum Enforcement {
  /** Report and continue. Production default. */
  Observe = 'observe',
  /** Report and reject. Dev, test and e2e. */
  Reject = 'reject',
}

export interface Violation {
  readonly operation: string;
  readonly direction: Direction;
  readonly schemaId: string;
  readonly schemaRevision: string;
  /** JSON Pointers into the offending instance. */
  readonly pointers: readonly string[];
  readonly messages: readonly string[];
}

export const summarize = (v: Violation): string =>
  `${v.operation} ${v.direction} violated ${v.schemaId} (${v.schemaRevision}): ${v.messages.join('; ')}`;

/** Effect boundary — implement over ores-otel. */
export interface ViolationSink {
  report(violation: Violation): void;
}

export const nullSink: ViolationSink = { report: () => undefined };

export type Verdict =
  | { readonly kind: 'valid' }
  | { readonly kind: 'skipped' }
  | { readonly kind: 'invalid'; readonly violation: Violation };

const VALID: Verdict = { kind: 'valid' };
const SKIPPED: Verdict = { kind: 'skipped' };

/**
 * Deterministic sampler. A given counter value always samples the same way,
 * so behaviour is reproducible in tests and consistent across a request.
 */
export class Sampler {
  private readonly rate: number;

  constructor(rate: number) {
    this.rate = Math.min(1, Math.max(0, rate));
  }

  static always(): Sampler {
    return new Sampler(1);
  }

  shouldSample(counter: number): boolean {
    if (this.rate >= 1) return true;
    if (this.rate <= 0) return false;
    const period = Math.max(1, Math.round(1 / this.rate));
    return counter % period === 0;
  }
}

export interface ContractSpec {
  readonly operation: string;
  readonly schemaId: string;
  readonly schemaRevision: string;
  /** A compiled ajv validator. Compiling is the caller's job, so this module
   *  stays a pure function of (validator, instance). */
  readonly validate: ValidateFunction;
}

/** Pure: same inputs, same verdict, no I/O. */
export const validate = (
  spec: ContractSpec,
  direction: Direction,
  instance: unknown,
): Verdict => {
  if (spec.validate(instance)) return VALID;
  const errors = spec.validate.errors ?? [];
  return {
    kind: 'invalid',
    violation: {
      operation: spec.operation,
      direction,
      schemaId: spec.schemaId,
      schemaRevision: spec.schemaRevision,
      pointers: errors.map((e) => e.instancePath),
      messages: errors.map((e) => `${e.instancePath || '/'} ${e.message ?? 'invalid'}`),
    },
  };
};

export class ContractViolationError extends Error {
  readonly violation: Violation;

  constructor(violation: Violation) {
    super(summarize(violation));
    this.name = 'ContractViolationError';
    this.violation = violation;
  }
}

export class Guard {
  constructor(
    private readonly spec: ContractSpec,
    private readonly enforcement: Enforcement,
    private readonly sampler: Sampler,
    private readonly sink: ViolationSink,
  ) {}

  /** Production posture: sample, observe, never reject. */
  static observing(spec: ContractSpec, rate: number, sink: ViolationSink): Guard {
    return new Guard(spec, Enforcement.Observe, new Sampler(rate), sink);
  }

  /** Dev/test/e2e posture: check everything, throw on violation. */
  static rejecting(spec: ContractSpec, sink: ViolationSink): Guard {
    return new Guard(spec, Enforcement.Reject, Sampler.always(), sink);
  }

  /**
   * Returns the verdict when the payload may proceed. Throws
   * ContractViolationError only under Enforcement.Reject.
   */
  check(direction: Direction, instance: unknown, counter: number): Verdict {
    if (!this.sampler.shouldSample(counter)) return SKIPPED;
    const verdict = validate(this.spec, direction, instance);
    if (verdict.kind !== 'invalid') return verdict;
    this.sink.report(verdict.violation);
    if (this.enforcement === Enforcement.Reject) {
      throw new ContractViolationError(verdict.violation);
    }
    return verdict;
  }
}
