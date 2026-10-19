use std::fmt;

/// Unique identifier for a migration run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MigrationId(pub String);

impl MigrationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MigrationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for MigrationId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for MigrationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a migration task in the task DAG.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TaskId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TaskId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a migration verification contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractId(pub String);

impl ContractId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ContractId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ContractId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a migration artifact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ArtifactId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ArtifactId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Monotonically increasing revision number for target or source artifacts.
///
/// Any mutation or new target artifact emission bumps this revision,
/// rendering any earlier target-revision evidence stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactRevision(pub u64);

impl ArtifactRevision {
    pub fn initial() -> Self {
        Self(1)
    }

    pub fn new(rev: u64) -> Self {
        Self(rev)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn is_initial(&self) -> bool {
        self.0 <= 1
    }
}

impl fmt::Display for ArtifactRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

impl From<u64> for ArtifactRevision {
    fn from(rev: u64) -> Self {
        Self(rev)
    }
}

/// Content-addressed cryptographic hash of an artifact or payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Computes deterministic 128-bit FNV-1a hex digest from raw bytes.
    pub fn compute(bytes: &[u8]) -> Self {
        const FNV_OFFSET_128: u128 = 0x6c62272e07bb014262b821756295c58d;
        const FNV_PRIME_128: u128 = 0x0000000001000000000000000000013b;

        let mut hash = FNV_OFFSET_128;
        for &byte in bytes {
            hash ^= byte as u128;
            hash = hash.wrapping_mul(FNV_PRIME_128);
        }
        Self(format!("{:032x}", hash))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Deterministic canonical hash representing a reviewed migration plan.
///
/// Any semantic change to source fingerprint, contract set, capability report,
/// migration plan, or adapter versions invalidates the reviewed plan hash.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanHash(pub String);

impl PlanHash {
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Computes deterministic canonical plan hash over its constituent parts.
    pub fn compute(
        source_fingerprint: &str,
        contract_ids: &[ContractId],
        capability_digest: &str,
        plan_digest: &str,
        adapter_versions: &str,
    ) -> Self {
        let mut sorted_contracts: Vec<&str> = contract_ids.iter().map(|c| c.as_str()).collect();
        sorted_contracts.sort_unstable();
        let contracts_joined = sorted_contracts.join(",");

        let canonical_repr = format!(
            "source:{}\ncontracts:{}\ncapability:{}\nplan:{}\nadapters:{}",
            source_fingerprint, contracts_joined, capability_digest, plan_digest, adapter_versions
        );
        let content_hash = ContentHash::compute(canonical_repr.as_bytes());
        Self(content_hash.0)
    }
}

impl fmt::Display for PlanHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
