#![forbid(unsafe_code)]
//! Runtime contract enforcement against JSON Schema.
//!
//! Compile-time types prove that *our* code builds. They prove nothing about
//! the bytes actually on the wire: a peer on an older schema revision, a
//! hand-rolled client, a proxy that helpfully "fixes" a field. This module
//! checks the payload itself, at runtime.
//!
//! JSON Schema is a **cross-check here, not the source of the types**. The
//! generated structs come from the primary IR (route map / `.cli-flags.toml`).
//! The schema is an independently-derived description of the same contract, so
//! when the two disagree, one of them has drifted — which is exactly the
//! signal worth having.
//!
//! Posture, matching how this is deployed:
//!   * dev / test / e2e — validate everything, and a violation is an error.
//!   * production       — validate a sample, report, never reject. A schema bug
//!                        must not become an outage.
//!
//! Effects are pushed outward: validation is a pure function of
//! `(schema, instance)`, and reporting goes through the `ViolationSink` trait
//! so the ores-otel wiring is injected rather than assumed.

use std::sync::Arc;

use serde_json::Value;

/// Where a payload was observed. Kept separate from the verdict so a sink can
/// route inbound and outbound violations differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Request,
    Response,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Request => "request",
            Direction::Response => "response",
        }
    }
}

/// How strictly to act on a failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    /// Report and continue. Production default.
    Observe,
    /// Report and reject. Dev, test and e2e.
    Reject,
}

/// A single contract violation, in a shape a log pipeline can index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub operation: String,
    pub direction: Direction,
    pub schema_id: String,
    pub schema_revision: String,
    /// JSON Pointers into the offending instance, most specific first.
    pub pointers: Vec<String>,
    pub messages: Vec<String>,
}

impl Violation {
    /// One-line summary for a log line; the structured fields carry the detail.
    pub fn summary(&self) -> String {
        format!(
            "{} {} violated {} ({}): {}",
            self.operation,
            self.direction.as_str(),
            self.schema_id,
            self.schema_revision,
            self.messages.join("; ")
        )
    }
}

/// Effect boundary. Implement over ores-otel; the guard itself stays pure.
pub trait ViolationSink: Send + Sync {
    fn report(&self, violation: &Violation);
}

/// A sink that drops everything. Useful in unit tests and as a safe default.
pub struct NullSink;

impl ViolationSink for NullSink {
    fn report(&self, _violation: &Violation) {}
}

/// Deterministic sampler, so a given operation samples consistently rather
/// than flickering. `rate` is clamped to `0.0..=1.0`; 1.0 always samples.
#[derive(Debug, Clone, Copy)]
pub struct Sampler {
    rate: f64,
}

impl Sampler {
    pub fn new(rate: f64) -> Self {
        Self { rate: rate.clamp(0.0, 1.0) }
    }

    pub fn always() -> Self {
        Self { rate: 1.0 }
    }

    /// `counter` is any monotonically increasing per-process value (a request
    /// sequence number). Avoids an RNG dependency and keeps the decision
    /// reproducible in tests.
    pub fn should_sample(&self, counter: u64) -> bool {
        if self.rate >= 1.0 {
            return true;
        }
        if self.rate <= 0.0 {
            return false;
        }
        let period = (1.0 / self.rate).round().max(1.0) as u64;
        counter % period == 0
    }
}

/// Outcome of a guard check. `Skipped` is not a pass — it records that no
/// opinion was formed, which matters when reading dashboards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    Skipped,
    Invalid(Violation),
}

impl Verdict {
    pub fn is_valid(&self) -> bool {
        matches!(self, Verdict::Valid)
    }
}

/// A compiled schema bound to the operation it describes.
pub struct Contract {
    pub operation: String,
    pub schema_id: String,
    pub schema_revision: String,
    compiled: jsonschema::Validator,
}

#[derive(Debug)]
pub enum ContractError {
    /// The schema document itself is not valid JSON Schema. This is always a
    /// build-time bug, never a runtime condition, so it is surfaced loudly.
    InvalidSchema(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::InvalidSchema(msg) => write!(f, "invalid JSON Schema: {msg}"),
        }
    }
}

impl std::error::Error for ContractError {}

impl Contract {
    pub fn compile(
        operation: impl Into<String>,
        schema_id: impl Into<String>,
        schema_revision: impl Into<String>,
        schema: &Value,
    ) -> Result<Self, ContractError> {
        let compiled = jsonschema::validator_for(schema)
            .map_err(|e| ContractError::InvalidSchema(e.to_string()))?;
        Ok(Self {
            operation: operation.into(),
            schema_id: schema_id.into(),
            schema_revision: schema_revision.into(),
            compiled,
        })
    }

    /// Pure: same inputs, same verdict, no I/O.
    pub fn validate(&self, direction: Direction, instance: &Value) -> Verdict {
        let mut pointers = Vec::new();
        let mut messages = Vec::new();
        for error in self.compiled.iter_errors(instance) {
            pointers.push(error.instance_path.to_string());
            messages.push(error.to_string());
        }
        if messages.is_empty() {
            return Verdict::Valid;
        }
        Verdict::Invalid(Violation {
            operation: self.operation.clone(),
            direction,
            schema_id: self.schema_id.clone(),
            schema_revision: self.schema_revision.clone(),
            pointers,
            messages,
        })
    }
}

/// Ties a contract to a posture and a sink.
pub struct Guard {
    contract: Arc<Contract>,
    enforcement: Enforcement,
    sampler: Sampler,
    sink: Arc<dyn ViolationSink>,
}

impl Guard {
    pub fn new(
        contract: Arc<Contract>,
        enforcement: Enforcement,
        sampler: Sampler,
        sink: Arc<dyn ViolationSink>,
    ) -> Self {
        Self { contract, enforcement, sampler, sink }
    }

    /// Production posture: sample, observe, never reject.
    pub fn observing(contract: Arc<Contract>, rate: f64, sink: Arc<dyn ViolationSink>) -> Self {
        Self::new(contract, Enforcement::Observe, Sampler::new(rate), sink)
    }

    /// Dev/test/e2e posture: check everything, reject on violation.
    pub fn rejecting(contract: Arc<Contract>, sink: Arc<dyn ViolationSink>) -> Self {
        Self::new(contract, Enforcement::Reject, Sampler::always(), sink)
    }

    /// Returns `Ok(verdict)` when the payload may proceed, `Err(violation)`
    /// when the caller must reject it. Reporting happens either way.
    pub fn check(
        &self,
        direction: Direction,
        instance: &Value,
        counter: u64,
    ) -> Result<Verdict, Violation> {
        if !self.sampler.should_sample(counter) {
            return Ok(Verdict::Skipped);
        }
        match self.contract.validate(direction, instance) {
            Verdict::Valid => Ok(Verdict::Valid),
            Verdict::Skipped => Ok(Verdict::Skipped),
            Verdict::Invalid(violation) => {
                self.sink.report(&violation);
                match self.enforcement {
                    Enforcement::Observe => Ok(Verdict::Invalid(violation)),
                    Enforcement::Reject => Err(violation),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn schema() -> Value {
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "revision"],
            "properties": {
                "id": {"type": "string", "minLength": 1},
                "revision": {"type": "string", "minLength": 1}
            }
        })
    }

    fn contract() -> Arc<Contract> {
        Arc::new(Contract::compile("GetFlags", "schema/v1/flagcatalog.json", "0001", &schema()).unwrap())
    }

    #[derive(Default)]
    struct Recording(Mutex<Vec<Violation>>);

    impl ViolationSink for Recording {
        fn report(&self, violation: &Violation) {
            self.0.lock().unwrap().push(violation.clone());
        }
    }

    #[test]
    fn valid_payload_passes_and_reports_nothing() {
        let sink = Arc::new(Recording::default());
        let guard = Guard::rejecting(contract(), sink.clone());
        let ok = serde_json::json!({"id": "a", "revision": "1"});
        assert_eq!(guard.check(Direction::Request, &ok, 0), Ok(Verdict::Valid));
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[test]
    fn reject_posture_returns_err_and_reports() {
        let sink = Arc::new(Recording::default());
        let guard = Guard::rejecting(contract(), sink.clone());
        let bad = serde_json::json!({"id": ""});
        assert!(guard.check(Direction::Request, &bad, 0).is_err());
        assert_eq!(sink.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn observe_posture_reports_but_lets_the_request_through() {
        let sink = Arc::new(Recording::default());
        let guard = Guard::observing(contract(), 1.0, sink.clone());
        let bad = serde_json::json!({"id": "", "revision": "1"});
        let verdict = guard.check(Direction::Response, &bad, 0).expect("must not reject");
        assert!(matches!(verdict, Verdict::Invalid(_)));
        assert_eq!(sink.0.lock().unwrap().len(), 1, "a fail-open guard must still report");
    }

    #[test]
    fn unknown_field_is_a_violation_not_a_shrug() {
        // additionalProperties:false is how schema drift shows up in practice.
        let sink = Arc::new(Recording::default());
        let guard = Guard::rejecting(contract(), sink);
        let drifted = serde_json::json!({"id": "a", "revision": "1", "tier": "gold"});
        assert!(guard.check(Direction::Request, &drifted, 0).is_err());
    }

    #[test]
    fn sampling_skips_without_claiming_validity() {
        let sink = Arc::new(Recording::default());
        let guard = Guard::observing(contract(), 0.1, sink.clone());
        let bad = serde_json::json!({"id": ""});
        // counter 1..9 are not sampled at rate 0.1 (period 10)
        assert_eq!(guard.check(Direction::Request, &bad, 3), Ok(Verdict::Skipped));
        assert!(sink.0.lock().unwrap().is_empty());
        // counter 10 is
        assert!(matches!(guard.check(Direction::Request, &bad, 10), Ok(Verdict::Invalid(_))));
    }

    #[test]
    fn sampler_rate_is_clamped() {
        assert!(Sampler::new(-1.0).should_sample(7) == false);
        assert!(Sampler::new(9.0).should_sample(7));
    }
}
