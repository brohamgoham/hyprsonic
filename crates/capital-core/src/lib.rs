//! Pure observation model and ports. No network, clock reads, files, or venue SDKs.
use serde::Serialize;
use std::fmt;
pub mod capital;
pub mod margin;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct EvmAddress(String);
impl EvmAddress {
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        if s.len() != 42 || !s.starts_with("0x") || !s[2..].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("expected a 0x-prefixed 20-byte address");
        }
        if s[2..].bytes().all(|b| b == b'0') {
            return Err("zero address is not an account");
        }
        Ok(Self(s.to_ascii_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact signed decimal. Unsupported precision/overflow is an error, never rounding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decimal {
    #[serde(serialize_with = "atoms_as_string")]
    atoms: i128,
    scale: u8,
}
fn atoms_as_string<S: serde::Serializer>(v: &i128, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}
impl Decimal {
    pub fn from_atoms(atoms: i128, scale: u8) -> Result<Self, &'static str> {
        if scale > 38 {
            return Err("unsupported decimal precision");
        }
        Ok(Self { atoms, scale })
    }
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        let digits = s.strip_prefix('-').unwrap_or(s);
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || fraction.len() > 38
            || (digits.contains('.') && fraction.is_empty())
        {
            return Err("invalid or unsupported decimal");
        }
        let mut atoms = 0i128;
        for b in whole.bytes().chain(fraction.bytes()) {
            atoms = atoms.checked_mul(10).ok_or("decimal overflow")?;
            atoms = if s.starts_with('-') {
                atoms.checked_sub((b - b'0') as i128)
            } else {
                atoms.checked_add((b - b'0') as i128)
            }
            .ok_or("decimal overflow")?;
        }
        Self::from_atoms(atoms, fraction.len() as u8)
    }
    pub fn atoms(&self) -> i128 {
        self.atoms
    }
    pub fn scale(&self) -> u8 {
        self.scale
    }
    pub fn zero() -> Self {
        Self { atoms: 0, scale: 0 }
    }
    fn aligned(&self, other: &Self) -> Result<(i128, i128, u8), &'static str> {
        let scale = self.scale.max(other.scale);
        let a = self
            .atoms
            .checked_mul(10i128.pow((scale - self.scale) as u32))
            .ok_or("decimal overflow")?;
        let b = other
            .atoms
            .checked_mul(10i128.pow((scale - other.scale) as u32))
            .ok_or("decimal overflow")?;
        Ok((a, b, scale))
    }
    pub fn checked_add(&self, other: &Self) -> Result<Self, &'static str> {
        let (a, b, s) = self.aligned(other)?;
        Self::from_atoms(a.checked_add(b).ok_or("decimal overflow")?, s)
    }
    pub fn checked_sub(&self, other: &Self) -> Result<Self, &'static str> {
        let (a, b, s) = self.aligned(other)?;
        Self::from_atoms(a.checked_sub(b).ok_or("decimal overflow")?, s)
    }
    pub fn compare(&self, other: &Self) -> Result<std::cmp::Ordering, &'static str> {
        let (a, b, _) = self.aligned(other)?;
        Ok(a.cmp(&b))
    }
    /// Nonnegative division rounded UP to a declared output precision.
    pub fn divide_ceil(&self, divisor: u32, scale: u8) -> Result<Self, &'static str> {
        if self.atoms < 0 || divisor == 0 || scale > 38 {
            return Err("invalid rounded division");
        }
        let (n, d) = if scale >= self.scale {
            (
                self.atoms
                    .checked_mul(10i128.pow((scale - self.scale) as u32))
                    .ok_or("decimal overflow")?,
                divisor as i128,
            )
        } else {
            (
                self.atoms,
                (divisor as i128)
                    .checked_mul(10i128.pow((self.scale - scale) as u32))
                    .ok_or("decimal overflow")?,
            )
        };
        Self::from_atoms(n / d + i128::from(n % d != 0), scale)
    }
}
impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = self.atoms.unsigned_abs().to_string();
        if self.scale > 0 {
            let scale = self.scale as usize;
            if s.len() <= scale {
                s = format!("{}{}", "0".repeat(scale + 1 - s.len()), s);
            }
            s.insert(s.len() - scale, '.');
        }
        write!(f, "{}{}", if self.atoms < 0 { "-" } else { "" }, s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AssetId {
    pub network: String,
    pub id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct AccountId {
    pub venue: String,
    pub address: EvmAddress,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CollateralDomain {
    Unresolved,
    VenueReported(String),
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FactValue {
    Amount(Decimal),
    Text(String),
    Flag(bool),
}
#[derive(Clone, Debug, Serialize)]
pub struct Fact {
    pub field: String,
    pub asset: Option<AssetId>,
    pub value: FactValue,
    pub evidence_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct EvidenceRef {
    pub id: String,
    pub endpoint: String,
    pub received_at_ms: u64,
    pub source_time_ms: Option<u64>,
    pub block: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Issue {
    pub code: String,
    pub scope: String,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadStatus {
    Complete,
    Partial,
    Unavailable,
}
#[derive(Clone, Debug, Serialize)]
pub struct AccountObservation {
    pub alias: String,
    pub account: AccountId,
    pub account_mode: Option<String>,
    pub collateral_domain: CollateralDomain,
    pub read_status: ReadStatus,
    pub facts: Vec<Fact>,
    pub evidence: Vec<EvidenceRef>,
    pub issues: Vec<Issue>,
    pub limitations: Vec<String>,
    pub margin_schedules: Vec<margin::MarginSchedule>,
    pub activities: Vec<capital::ObservedActivity>,
}
impl AccountObservation {
    pub fn new(alias: String, venue: &str, address: EvmAddress) -> Self {
        Self {
            alias,
            account: AccountId {
                venue: venue.into(),
                address,
            },
            account_mode: None,
            collateral_domain: CollateralDomain::Unresolved,
            read_status: ReadStatus::Unavailable,
            facts: vec![],
            evidence: vec![],
            issues: vec![],
            limitations: vec![
                "Raw observations alone do not establish funding eligibility.".into(),
                "Public data cannot establish account control or all private obligations.".into(),
            ],
            margin_schedules: vec![],
            activities: vec![],
        }
    }
    pub fn issue(&mut self, code: &str, scope: &str, detail: &str) {
        self.issues.push(Issue {
            code: code.into(),
            scope: scope.into(),
            detail: detail.into(),
        });
    }
    pub fn finish(&mut self, now_ms: u64, max_age_ms: u64) {
        let stale = self.evidence.iter().any(|e| {
            e.source_time_ms
                .unwrap_or(e.received_at_ms)
                .checked_add(max_age_ms)
                .is_none_or(|deadline| now_ms > deadline)
        });
        let future = self.evidence.iter().any(|e| {
            e.source_time_ms
                .is_some_and(|t| t > now_ms.saturating_add(5000))
        });
        if stale {
            self.issue(
                "STALE",
                "observation",
                "Required evidence exceeds the configured age bound.",
            );
        }
        if future {
            self.issue(
                "CLOCK_SKEW",
                "observation",
                "Source time is ahead of the observation clock.",
            );
        }
        self.read_status = if self.evidence.is_empty() {
            ReadStatus::Unavailable
        } else if self.issues.is_empty() {
            ReadStatus::Complete
        } else {
            ReadStatus::Partial
        };
    }
}

pub trait Clock: Sync {
    fn now_ms(&self) -> u64;
}
pub trait EvidenceStore: Sync {
    fn append(&self, record: &[u8]) -> Result<String, String>;
}
pub trait AccountSource: Sync {
    fn observe(&self) -> Result<AccountObservation, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_is_exact_and_bounded() {
        for s in [
            "0.000001",
            "123456789123456789.123456",
            "-0.01",
            "-170141183460469231731687303715884105728",
        ] {
            assert_eq!(Decimal::parse(s).unwrap().to_string(), s);
        }
        for s in [
            "NaN",
            "1e8",
            "1.2.3",
            "1.",
            "+1",
            " 1",
            "170141183460469231731687303715884105728",
        ] {
            assert!(Decimal::parse(s).is_err(), "{s}");
        }
        assert!(Decimal::from_atoms(1, 39).is_err());
    }
    #[test]
    fn address_rejects_placeholders() {
        assert!(EvmAddress::parse("YOUR_ADDRESS").is_err());
        assert!(EvmAddress::parse("0x0000000000000000000000000000000000000000").is_err());
    }
    #[test]
    fn stale_and_future_sources_are_not_complete() {
        let mut o = AccountObservation::new(
            "a".into(),
            "x",
            EvmAddress::parse("0x1111111111111111111111111111111111111111").unwrap(),
        );
        o.evidence.push(EvidenceRef {
            id: "1".into(),
            endpoint: "test".into(),
            received_at_ms: 100,
            source_time_ms: Some(1),
            block: None,
        });
        o.finish(100, 10);
        assert!(matches!(o.read_status, ReadStatus::Partial));
        o.issues.clear();
        o.evidence[0].source_time_ms = Some(10000);
        o.finish(100, 10);
        assert!(o.issues.iter().any(|i| i.code == "CLOCK_SKEW"));
    }
}
