/// Runtime contract enforcement against JSON Schema.
///
/// Port of `runtime/rust/contract_guard.rs`, with the same posture and the
/// same reported field names, so a violation from a Flutter client and one
/// from a Rust service are the same record in ores-otel.
///
/// JSON Schema is a cross-check here, not the source of the generated types.
/// When the two disagree, one of them has drifted.
///
///   dev / test / e2e  validate everything, throw on violation
///   production        validate a sample, report, never throw
library;

/// Where a payload was observed.
enum Direction {
  request('request'),
  response('response');

  const Direction(this.wire);
  final String wire;
}

/// How strictly to act on a failed validation.
enum Enforcement {
  /// Report and continue. Production default.
  observe,

  /// Report and throw. Dev, test and e2e.
  reject,
}

/// A single contract violation, shaped for a log pipeline.
class Violation {
  const Violation({
    required this.operation,
    required this.direction,
    required this.schemaId,
    required this.schemaRevision,
    required this.pointers,
    required this.messages,
  });

  final String operation;
  final Direction direction;
  final String schemaId;
  final String schemaRevision;

  /// JSON Pointers into the offending instance.
  final List<String> pointers;
  final List<String> messages;

  String get summary =>
      '$operation ${direction.wire} violated $schemaId '
      '($schemaRevision): ${messages.join('; ')}';

  Map<String, Object?> toJson() => {
        'operation': operation,
        'direction': direction.wire,
        'schema_id': schemaId,
        'schema_revision': schemaRevision,
        'pointers': pointers,
        'messages': messages,
      };
}

/// Effect boundary — implement over ores-otel.
abstract interface class ViolationSink {
  void report(Violation violation);
}

/// Drops everything. Safe default, and what unit tests use.
class NullSink implements ViolationSink {
  const NullSink();

  @override
  void report(Violation violation) {}
}

/// Outcome of a check. `skipped` is not a pass — it records that no opinion
/// was formed, which matters when reading a dashboard.
sealed class Verdict {
  const Verdict();
}

class Valid extends Verdict {
  const Valid();
}

class Skipped extends Verdict {
  const Skipped();
}

class Invalid extends Verdict {
  const Invalid(this.violation);
  final Violation violation;
}

/// Deterministic sampler, so a given counter always samples the same way.
class Sampler {
  Sampler(double rate) : rate = rate.clamp(0.0, 1.0);
  Sampler.always() : rate = 1.0;

  final double rate;

  bool shouldSample(int counter) {
    if (rate >= 1.0) return true;
    if (rate <= 0.0) return false;
    final period = (1.0 / rate).round().clamp(1, 1 << 30);
    return counter % period == 0;
  }
}

/// Validates an instance and returns the failures. Supplied by the caller so
/// this library stays independent of any one JSON Schema package.
typedef SchemaValidator = List<String> Function(Object? instance);

class ContractSpec {
  const ContractSpec({
    required this.operation,
    required this.schemaId,
    required this.schemaRevision,
    required this.validator,
  });

  final String operation;
  final String schemaId;
  final String schemaRevision;
  final SchemaValidator validator;
}

class ContractViolationException implements Exception {
  const ContractViolationException(this.violation);
  final Violation violation;

  @override
  String toString() => 'ContractViolationException: ${violation.summary}';
}

/// Pure: same inputs, same verdict, no I/O.
Verdict validateInstance(
  ContractSpec spec,
  Direction direction,
  Object? instance,
) {
  final failures = spec.validator(instance);
  if (failures.isEmpty) return const Valid();
  return Invalid(
    Violation(
      operation: spec.operation,
      direction: direction,
      schemaId: spec.schemaId,
      schemaRevision: spec.schemaRevision,
      pointers: const [],
      messages: failures,
    ),
  );
}

class Guard {
  Guard({
    required this.spec,
    required this.enforcement,
    required this.sampler,
    required this.sink,
  });

  /// Production posture: sample, observe, never throw.
  Guard.observing(this.spec, double rate, this.sink)
      : enforcement = Enforcement.observe,
        sampler = Sampler(rate);

  /// Dev/test/e2e posture: check everything, throw on violation.
  Guard.rejecting(this.spec, this.sink)
      : enforcement = Enforcement.reject,
        sampler = Sampler.always();

  final ContractSpec spec;
  final Enforcement enforcement;
  final Sampler sampler;
  final ViolationSink sink;

  /// Returns the verdict when the payload may proceed. Throws
  /// [ContractViolationException] only under [Enforcement.reject].
  Verdict check(Direction direction, Object? instance, int counter) {
    if (!sampler.shouldSample(counter)) return const Skipped();
    final verdict = validateInstance(spec, direction, instance);
    if (verdict is! Invalid) return verdict;
    sink.report(verdict.violation);
    if (enforcement == Enforcement.reject) {
      throw ContractViolationException(verdict.violation);
    }
    return verdict;
  }
}
