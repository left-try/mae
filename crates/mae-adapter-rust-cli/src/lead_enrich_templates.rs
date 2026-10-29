//! Multi-module Rust templates for the lead-enrichment deterministic CLI pipeline slice.
//!
//! Enforces strict Logic-CLI separation across:
//! - `src/cli.rs`: Argument parsing and subcommand dispatch (`ingest`, `enrich`, `report`)
//! - `src/core/ingest.rs`: Schema validation, email/domain normalization, and deduplication
//! - `src/core/cache.rs`: Persistent state and domain cache ledger I/O
//! - `src/core/retry.rs`: Bounded retry policy and transient failure diagnostics
//! - `src/core/enrich.rs`: Enrichment pipeline stage with cache, retry, and checkpoint resume
//! - `src/core/report.rs`: Stage outcome summary and JSON reporting

pub const LEAD_CORE_MOD_RS: &str = r#"//! Pure domain logic for the lead-enrichment CLI pipeline.

pub mod cache;
pub mod data;
pub mod enrich;
pub mod ingest;
pub mod report;
pub mod retry;
pub mod sha256;
"#;

pub const LEAD_INGEST_RS: &str = r#"//! Lead ingestion, schema validation, email/domain normalization, and deduplication.

use std::collections::{BTreeMap, BTreeSet};
use crate::core::data::{JsonValue, Record};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeadRecord {
    pub id: String,
    pub email: String,
    pub domain: String,
    pub company: String,
    pub role: String,
}

impl LeadRecord {
    pub fn to_json_object(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("company".to_string(), JsonValue::String(self.company.clone()));
        map.insert("domain".to_string(), JsonValue::String(self.domain.clone()));
        map.insert("email".to_string(), JsonValue::String(self.email.clone()));
        map.insert("id".to_string(), JsonValue::String(self.id.clone()));
        map.insert("role".to_string(), JsonValue::String(self.role.clone()));
        JsonValue::Object(map)
    }
}

pub fn normalize_leads(raw_records: &[Record], dedupe: bool) -> Result<(Vec<LeadRecord>, usize), i32> {
    let mut normalized = Vec::new();
    let mut seen_emails = BTreeSet::new();
    let mut duplicates_removed = 0usize;

    for (idx, rec) in raw_records.iter().enumerate() {
        let id = rec
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let email = rec
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let company = rec
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let role = rec
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if id.is_empty() || email.is_empty() || !email.contains('@') {
            eprintln!(
                "ERROR [DATA]: invalid lead record at index {} (id='{}', email='{}')",
                idx, id, email
            );
            return Err(3);
        }

        let domain = email.split_once('@').map(|(_, d)| d.trim().to_string()).unwrap_or_default();
        if domain.is_empty() || !domain.contains('.') {
            eprintln!(
                "ERROR [DATA]: invalid email domain at index {}: '{}'",
                idx, email
            );
            return Err(3);
        }

        if dedupe {
            if seen_emails.contains(&email) {
                duplicates_removed += 1;
                continue;
            }
            seen_emails.insert(email.clone());
        }

        normalized.push(LeadRecord {
            id,
            email,
            domain,
            company,
            role,
        });
    }

    Ok((normalized, duplicates_removed))
}

pub fn run_ingest(raw_records: &[Record], dedupe: bool) -> Result<JsonValue, i32> {
    let (leads, duplicates_removed) = normalize_leads(raw_records, dedupe)?;
    let valid_count = leads.len();
    let leads_json: Vec<JsonValue> = leads.iter().map(|l| l.to_json_object()).collect();

    let mut result = BTreeMap::new();
    result.insert(
        "duplicates_removed".to_string(),
        JsonValue::Integer(duplicates_removed as i64),
    );
    result.insert("leads".to_string(), JsonValue::Array(leads_json));
    result.insert("stage".to_string(), JsonValue::String("ingest".to_string()));
    result.insert(
        "total_input".to_string(),
        JsonValue::Integer(raw_records.len() as i64),
    );
    result.insert(
        "valid_count".to_string(),
        JsonValue::Integer(valid_count as i64),
    );

    Ok(JsonValue::Object(result))
}

#[cfg(test)]
#[test]
fn test_normalize_and_dedupe_leads() {
    let mut r1 = BTreeMap::new();
    r1.insert("id".to_string(), JsonValue::String("lead-1".to_string()));
    r1.insert("email".to_string(), JsonValue::String(" Alice@Acme.io ".to_string()));
    r1.insert("company".to_string(), JsonValue::String("Acme".to_string()));
    r1.insert("role".to_string(), JsonValue::String("VP".to_string()));

    let mut r2 = BTreeMap::new();
    r2.insert("id".to_string(), JsonValue::String("lead-2".to_string()));
    r2.insert("email".to_string(), JsonValue::String("alice@acme.io".to_string()));
    r2.insert("company".to_string(), JsonValue::String("Acme Dup".to_string()));
    r2.insert("role".to_string(), JsonValue::String("VP".to_string()));

    let (leads, dupes) = normalize_leads(&[r1, r2], true).unwrap();
    assert_eq!(leads.len(), 1);
    assert_eq!(dupes, 1);
    assert_eq!(leads[0].email, "alice@acme.io");
    assert_eq!(leads[0].domain, "acme.io");
}
"#;

pub const LEAD_CACHE_RS: &str = r#"//! Persistent state and domain cache ledger management.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::core::data::{parse_json_doc, JsonValue, Record};
use crate::core::sha256::sha256_hex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainProfile {
    pub domain: String,
    pub employee_band: String,
    pub enrichment_hash: String,
    pub lead_score: i64,
    pub tier: String,
}

impl DomainProfile {
    pub fn compute(domain: &str) -> Self {
        let digest = sha256_hex(domain.as_bytes());
        let prefix_val = u32::from_str_radix(&digest[..4], 16).unwrap_or(0);
        let score = ((prefix_val % 50) + 50) as i64;
        let tier = if score >= 80 {
            "enterprise"
        } else if score >= 65 {
            "mid_market"
        } else {
            "growth"
        };
        let employee_band = if tier == "enterprise" {
            "250-1000"
        } else {
            "50-250"
        };

        Self {
            domain: domain.to_string(),
            employee_band: employee_band.to_string(),
            enrichment_hash: digest[..12].to_string(),
            lead_score: score,
            tier: tier.to_string(),
        }
    }

    pub fn to_json_object(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("domain".to_string(), JsonValue::String(self.domain.clone()));
        map.insert(
            "employee_band".to_string(),
            JsonValue::String(self.employee_band.clone()),
        );
        map.insert(
            "enrichment_hash".to_string(),
            JsonValue::String(self.enrichment_hash.clone()),
        );
        map.insert("lead_score".to_string(), JsonValue::Integer(self.lead_score));
        map.insert("tier".to_string(), JsonValue::String(self.tier.clone()));
        JsonValue::Object(map)
    }

    pub fn from_json_object(obj: &Record) -> Option<Self> {
        Some(Self {
            domain: obj.get("domain")?.as_str()?.to_string(),
            employee_band: obj.get("employee_band")?.as_str()?.to_string(),
            enrichment_hash: obj.get("enrichment_hash")?.as_str()?.to_string(),
            lead_score: obj.get("lead_score")?.as_i64()?,
            tier: obj.get("tier")?.as_str()?.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStateLedger {
    pub cache_hits: i64,
    pub completed_ids: Vec<String>,
    pub domain_cache: BTreeMap<String, DomainProfile>,
    pub enriched_leads: Vec<JsonValue>,
    pub failed_ids: Vec<String>,
    pub resumed_skipped: i64,
    pub retried_count: i64,
    pub stage_status: String,
}

impl Default for PipelineStateLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineStateLedger {
    pub fn new() -> Self {
        Self {
            cache_hits: 0,
            completed_ids: Vec::new(),
            domain_cache: BTreeMap::new(),
            enriched_leads: Vec::new(),
            failed_ids: Vec::new(),
            resumed_skipped: 0,
            retried_count: 0,
            stage_status: "in_progress".to_string(),
        }
    }

    pub fn load_for_resume(path: &str) -> Result<Self, i32> {
        let doc = load_state_json(path)?;
        let obj = doc.as_object().ok_or(3)?;
        let mut state = Self::new();

        if let Some(JsonValue::Object(cache_map)) = obj.get("domain_cache") {
            for (k, v) in cache_map {
                if let Some(rec) = v.as_object() {
                    if let Some(profile) = DomainProfile::from_json_object(rec) {
                        state.domain_cache.insert(k.clone(), profile);
                    }
                }
            }
        }

        if let Some(JsonValue::Array(arr)) = obj.get("completed_ids") {
            for item in arr {
                if let Some(s) = item.as_str() {
                    state.completed_ids.push(s.to_string());
                }
            }
        }

        if let Some(JsonValue::Array(arr)) = obj.get("enriched_leads") {
            state.enriched_leads = arr.clone();
        }

        Ok(state)
    }

    pub fn to_json_value(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("cache_hits".to_string(), JsonValue::Integer(self.cache_hits));
        map.insert(
            "completed_ids".to_string(),
            JsonValue::Array(
                self.completed_ids
                    .iter()
                    .map(|s| JsonValue::String(s.clone()))
                    .collect(),
            ),
        );

        let mut dc = BTreeMap::new();
        for (k, v) in &self.domain_cache {
            dc.insert(k.clone(), v.to_json_object());
        }
        map.insert("domain_cache".to_string(), JsonValue::Object(dc));
        map.insert(
            "enriched_leads".to_string(),
            JsonValue::Array(self.enriched_leads.clone()),
        );
        map.insert(
            "failed_ids".to_string(),
            JsonValue::Array(
                self.failed_ids
                    .iter()
                    .map(|s| JsonValue::String(s.clone()))
                    .collect(),
            ),
        );
        map.insert(
            "resumed_skipped".to_string(),
            JsonValue::Integer(self.resumed_skipped),
        );
        map.insert(
            "retried_count".to_string(),
            JsonValue::Integer(self.retried_count),
        );
        map.insert(
            "stage_status".to_string(),
            JsonValue::String(self.stage_status.clone()),
        );
        JsonValue::Object(map)
    }

    pub fn save(&self, path: &str) -> Result<(), i32> {
        write_json_file(path, &self.to_json_value())
    }
}

pub fn load_state_json(path: &str) -> Result<JsonValue, i32> {
    if path.is_empty() || !Path::new(path).exists() {
        eprintln!("ERROR [IO]: state file not found: {}", path);
        return Err(2);
    }
    let content = fs::read_to_string(path).map_err(|_| 2)?;
    parse_json_doc(&content)
}

pub fn write_json_file(path: &str, value: &JsonValue) -> Result<(), i32> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| 2)?;
        }
    }
    fs::write(path, format!("{}\n", value.to_pretty_string(0))).map_err(|_| 2)
}
"#;

pub const LEAD_RETRY_RS: &str = r#"//! Bounded retry policy for transient provider errors during enrichment.

/// Simulates bounded retry execution against a provider domain.
/// Returns `Ok(retries_used)` if enrichment succeeds within `max_retries`,
/// or `Err(retries_used)` if all retry attempts are exhausted.
pub fn execute_with_retry(domain: &str, fail_domain: &str, max_retries: i32) -> Result<i64, i64> {
    if fail_domain.is_empty() || domain != fail_domain {
        return Ok(0);
    }

    let mut attempts = 0;
    let mut retried_count = 0i64;
    let mut succeeded = false;

    while attempts <= max_retries {
        attempts += 1;
        if attempts >= 3 {
            succeeded = true;
            break;
        }
        retried_count += 1;
        eprintln!(
            "WARN [RETRY]: transient provider failure for domain='{}' attempt={}/{}",
            domain,
            attempts,
            max_retries + 1
        );
    }

    if succeeded {
        Ok(retried_count)
    } else {
        Err(retried_count)
    }
}

#[cfg(test)]
#[test]
fn test_retry_policy_recovery_and_exhaustion() {
    // Succeeds on attempt 3 when max_retries >= 2
    assert_eq!(execute_with_retry("flaky.org", "flaky.org", 2), Ok(2));
    // Fails when max_retries < 2
    assert_eq!(execute_with_retry("flaky.org", "flaky.org", 1), Err(2));
}
"#;

pub const LEAD_ENRICH_RS: &str = r#"//! Enrichment stage coordinating domain cache, retry policy, and checkpoint resume.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::core::cache::{DomainProfile, PipelineStateLedger};
use crate::core::data::{JsonValue, Record};
use crate::core::ingest::normalize_leads;
use crate::core::retry::execute_with_retry;

pub fn run_enrich(
    raw_records: &[Record],
    state_file: Option<&str>,
    max_retries: i32,
    fail_domain: &str,
    resume: bool,
) -> Result<JsonValue, i32> {
    if max_retries < 0 {
        eprintln!("ERROR [ARG]: --max-retries must be non-negative");
        return Err(1);
    }

    let (leads, _) = normalize_leads(raw_records, false)?;

    let mut state = if resume {
        if let Some(sf) = state_file.filter(|s| !s.is_empty() && Path::new(s).exists()) {
            PipelineStateLedger::load_for_resume(sf)?
        } else {
            PipelineStateLedger::new()
        }
    } else {
        PipelineStateLedger::new()
    };

    let mut completed_set: BTreeSet<String> = state.completed_ids.iter().cloned().collect();

    for lead in leads {
        if resume && completed_set.contains(&lead.id) {
            state.resumed_skipped += 1;
            continue;
        }

        let profile = if let Some(cached) = state.domain_cache.get(&lead.domain).cloned() {
            state.cache_hits += 1;
            cached
        } else {
            match execute_with_retry(&lead.domain, fail_domain, max_retries) {
                Ok(retries) => {
                    state.retried_count += retries;
                }
                Err(retries) => {
                    state.retried_count += retries;
                    state.failed_ids.push(lead.id.clone());
                    state.stage_status = "failed".to_string();
                    if let Some(sf) = state_file.filter(|s| !s.is_empty()) {
                        let _ = state.save(sf);
                    }
                    eprintln!(
                        "ERROR [STAGE]: retries exhausted for domain='{}' lead_id='{}'",
                        lead.domain, lead.id
                    );
                    return Err(4);
                }
            }

            let computed = DomainProfile::compute(&lead.domain);
            state.domain_cache.insert(lead.domain.clone(), computed.clone());
            computed
        };

        let mut enriched_rec = BTreeMap::new();
        enriched_rec.insert("company".to_string(), JsonValue::String(lead.company));
        enriched_rec.insert("domain".to_string(), JsonValue::String(lead.domain));
        enriched_rec.insert("email".to_string(), JsonValue::String(lead.email));
        enriched_rec.insert(
            "enrichment_hash".to_string(),
            JsonValue::String(profile.enrichment_hash),
        );
        enriched_rec.insert("id".to_string(), JsonValue::String(lead.id.clone()));
        enriched_rec.insert(
            "lead_score".to_string(),
            JsonValue::Integer(profile.lead_score),
        );
        enriched_rec.insert("role".to_string(), JsonValue::String(lead.role));
        enriched_rec.insert("tier".to_string(), JsonValue::String(profile.tier));

        state.enriched_leads.push(JsonValue::Object(enriched_rec));
        state.completed_ids.push(lead.id.clone());
        completed_set.insert(lead.id);
    }

    state.stage_status = "completed".to_string();
    if let Some(sf) = state_file.filter(|s| !s.is_empty()) {
        state.save(sf)?;
    }

    let mut out = BTreeMap::new();
    out.insert(
        "cache_hits".to_string(),
        JsonValue::Integer(state.cache_hits),
    );
    out.insert(
        "completed_count".to_string(),
        JsonValue::Integer(state.completed_ids.len() as i64),
    );
    out.insert(
        "enriched_leads".to_string(),
        JsonValue::Array(state.enriched_leads),
    );
    out.insert(
        "failed_count".to_string(),
        JsonValue::Integer(state.failed_ids.len() as i64),
    );
    out.insert(
        "resumed_skipped".to_string(),
        JsonValue::Integer(state.resumed_skipped),
    );
    out.insert(
        "retried_count".to_string(),
        JsonValue::Integer(state.retried_count),
    );
    out.insert("stage".to_string(), JsonValue::String("enrich".to_string()));
    out.insert(
        "stage_status".to_string(),
        JsonValue::String(state.stage_status),
    );

    Ok(JsonValue::Object(out))
}
"#;

pub const LEAD_REPORT_RS: &str = r#"//! Stage outcome reporting from persistent state ledger.

use std::collections::BTreeMap;

use crate::core::cache::load_state_json;
use crate::core::data::JsonValue;

pub fn run_report(state_file: &str, format: &str) -> Result<String, i32> {
    let doc = load_state_json(state_file)?;
    let obj = doc.as_object().ok_or(3)?;

    let completed_count = obj
        .get("completed_ids")
        .and_then(|v| match v {
            JsonValue::Array(a) => Some(a.len() as i64),
            _ => None,
        })
        .unwrap_or(0);
    let failed_count = obj
        .get("failed_ids")
        .and_then(|v| match v {
            JsonValue::Array(a) => Some(a.len() as i64),
            _ => None,
        })
        .unwrap_or(0);
    let cache_hits = obj.get("cache_hits").and_then(|v| v.as_i64()).unwrap_or(0);
    let retried_count = obj
        .get("retried_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let resumed_skipped = obj
        .get("resumed_skipped")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let stage_status = obj
        .get("stage_status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if format == "summary" {
        Ok(format!(
            "STAGE_REPORT status={} completed={} failed={} cache_hits={} retries={} resumed_skipped={}",
            stage_status, completed_count, failed_count, cache_hits, retried_count, resumed_skipped
        ))
    } else {
        let mut report = BTreeMap::new();
        report.insert("cache_hits".to_string(), JsonValue::Integer(cache_hits));
        report.insert(
            "completed_count".to_string(),
            JsonValue::Integer(completed_count),
        );
        report.insert("failed_count".to_string(), JsonValue::Integer(failed_count));
        report.insert(
            "resumed_skipped".to_string(),
            JsonValue::Integer(resumed_skipped),
        );
        report.insert(
            "retried_count".to_string(),
            JsonValue::Integer(retried_count),
        );
        report.insert("stage".to_string(), JsonValue::String("report".to_string()));
        report.insert(
            "stage_status".to_string(),
            JsonValue::String(stage_status.to_string()),
        );
        Ok(JsonValue::Object(report).to_pretty_string(0))
    }
}
"#;

pub const LEAD_CLI_RS: &str = r#"//! Pure CLI argument parser for the lead_enrich pipeline utility.

#[derive(Debug, Clone)]
pub enum Subcommand {
    Ingest {
        input: String,
        output: Option<String>,
        dedupe: bool,
    },
    Enrich {
        input: String,
        state_file: Option<String>,
        output: Option<String>,
        max_retries: i32,
        fail_domain: String,
        resume: bool,
    },
    Report {
        state_file: String,
        format: String,
    },
}

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub verbose: bool,
    pub quiet: bool,
    pub subcommand: Subcommand,
}

pub fn parse_args(args: &[String]) -> Result<CliArgs, i32> {
    let mut verbose = false;
    let mut quiet = false;
    let mut idx = 0;

    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--version" || arg == "-V" {
            println!("lead_enrich 1.2.0");
            std::process::exit(0);
        } else if arg == "--help" || arg == "-h" {
            println!("usage: lead_enrich [--version] [-v] [-q] {{ingest,enrich,report}} ...");
            std::process::exit(0);
        } else if arg == "-v" || arg == "--verbose" {
            verbose = true;
            idx += 1;
        } else if arg == "-q" || arg == "--quiet" {
            quiet = true;
            idx += 1;
        } else if arg.starts_with('-') {
            eprintln!("ERROR [ARG]: unrecognized global option '{}'", arg);
            return Err(1);
        } else {
            break;
        }
    }

    if idx >= args.len() {
        eprintln!("ERROR [ARG]: missing required subcommand (choose from: ingest, enrich, report)");
        return Err(1);
    }

    let sub = &args[idx];
    let rem = &args[idx + 1..];

    let subcommand = match sub.as_str() {
        "ingest" => parse_ingest(rem)?,
        "enrich" => parse_enrich(rem)?,
        "report" => parse_report(rem)?,
        other => {
            eprintln!("ERROR [ARG]: unknown subcommand '{}'", other);
            return Err(1);
        }
    };

    Ok(CliArgs {
        verbose,
        quiet,
        subcommand,
    })
}

fn parse_ingest(args: &[String]) -> Result<Subcommand, i32> {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut dedupe = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --output");
                    1
                })?;
                if !val.is_empty() {
                    output = Some(val.clone());
                }
            }
            "--dedupe" => {
                dedupe = true;
            }
            flag if flag.starts_with('-') => {
                eprintln!("ERROR [ARG]: unknown option for ingest: {}", flag);
                return Err(1);
            }
            pos => {
                if input.is_none() {
                    input = Some(pos.to_string());
                } else {
                    eprintln!("ERROR [ARG]: unexpected positional argument '{}'", pos);
                    return Err(1);
                }
            }
        }
        i += 1;
    }

    let input = input.ok_or_else(|| {
        eprintln!("ERROR [ARG]: ingest requires <input> path");
        1
    })?;

    Ok(Subcommand::Ingest {
        input,
        output,
        dedupe,
    })
}

fn parse_enrich(args: &[String]) -> Result<Subcommand, i32> {
    let mut input: Option<String> = None;
    let mut state_file: Option<String> = None;
    let mut output: Option<String> = None;
    let mut max_retries: i32 = 2;
    let mut fail_domain = String::new();
    let mut resume = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--state-file" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --state-file");
                    1
                })?;
                if !val.is_empty() {
                    state_file = Some(val.clone());
                }
            }
            "-o" | "--output" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --output");
                    1
                })?;
                if !val.is_empty() {
                    output = Some(val.clone());
                }
            }
            "-r" | "--max-retries" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --max-retries");
                    1
                })?;
                max_retries = val.parse::<i32>().map_err(|_| {
                    eprintln!("ERROR [ARG]: invalid integer for --max-retries: '{}'", val);
                    1
                })?;
            }
            "-f" | "--fail-domain" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --fail-domain");
                    1
                })?;
                fail_domain = val.clone();
            }
            "--resume" => {
                resume = true;
            }
            flag if flag.starts_with('-') => {
                eprintln!("ERROR [ARG]: unknown option for enrich: {}", flag);
                return Err(1);
            }
            pos => {
                if input.is_none() {
                    input = Some(pos.to_string());
                } else {
                    eprintln!("ERROR [ARG]: unexpected positional argument '{}'", pos);
                    return Err(1);
                }
            }
        }
        i += 1;
    }

    let input = input.ok_or_else(|| {
        eprintln!("ERROR [ARG]: enrich requires <input> path");
        1
    })?;

    Ok(Subcommand::Enrich {
        input,
        state_file,
        output,
        max_retries,
        fail_domain,
        resume,
    })
}

fn parse_report(args: &[String]) -> Result<Subcommand, i32> {
    let mut state_file: Option<String> = None;
    let mut format = "json".to_string();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--format" => {
                i += 1;
                let val = args.get(i).ok_or_else(|| {
                    eprintln!("ERROR [ARG]: missing value for --format");
                    1
                })?;
                if val != "json" && val != "summary" {
                    eprintln!("ERROR [ARG]: invalid format '{}' (choose json or summary)", val);
                    return Err(1);
                }
                format = val.clone();
            }
            flag if flag.starts_with('-') => {
                eprintln!("ERROR [ARG]: unknown option for report: {}", flag);
                return Err(1);
            }
            pos => {
                if state_file.is_none() {
                    state_file = Some(pos.to_string());
                } else {
                    eprintln!("ERROR [ARG]: unexpected positional argument '{}'", pos);
                    return Err(1);
                }
            }
        }
        i += 1;
    }

    let state_file = state_file.ok_or_else(|| {
        eprintln!("ERROR [ARG]: report requires <state_file> path");
        1
    })?;

    Ok(Subcommand::Report { state_file, format })
}
"#;

pub const LEAD_MAIN_RS: &str = r#"//! Entrypoint for the lead_enrich native Rust CLI utility.

mod cli;
mod core;

use crate::core::cache::write_json_file;
use crate::core::data::read_lead_records;

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let args = match cli::parse_args(&raw_args) {
        Ok(a) => a,
        Err(code) => std::process::exit(code),
    };
    let _ = (args.verbose, args.quiet);

    let exit_code = match args.subcommand {
        cli::Subcommand::Ingest {
            input,
            output,
            dedupe,
        } => {
            let records = match read_lead_records(&input) {
                Ok(r) => r,
                Err(code) => std::process::exit(code),
            };
            let result = match core::ingest::run_ingest(&records, dedupe) {
                Ok(v) => v,
                Err(code) => std::process::exit(code),
            };
            if let Some(ref out_path) = output {
                if let Err(code) = write_json_file(out_path, &result) {
                    std::process::exit(code);
                }
            }
            println!("{}", result.to_pretty_string(0));
            0
        }
        cli::Subcommand::Enrich {
            input,
            state_file,
            output,
            max_retries,
            fail_domain,
            resume,
        } => {
            let records = match read_lead_records(&input) {
                Ok(r) => r,
                Err(code) => std::process::exit(code),
            };
            let result = match core::enrich::run_enrich(
                &records,
                state_file.as_deref(),
                max_retries,
                &fail_domain,
                resume,
            ) {
                Ok(v) => v,
                Err(code) => std::process::exit(code),
            };
            if let Some(ref out_path) = output {
                if let Err(code) = write_json_file(out_path, &result) {
                    std::process::exit(code);
                }
            }
            println!("{}", result.to_pretty_string(0));
            0
        }
        cli::Subcommand::Report { state_file, format } => {
            match core::report::run_report(&state_file, &format) {
                Ok(text) => {
                    println!("{}", text);
                    0
                }
                Err(code) => code,
            }
        }
    };

    std::process::exit(exit_code);
}
"#;
