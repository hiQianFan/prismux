use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Availability {
    pub state: AvailabilityState,
    pub display: String,
}

impl Availability {
    pub fn unknown() -> Self {
        Self {
            state: AvailabilityState::Unknown,
            display: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AvailabilityState {
    Unknown,
    Available,
    Limited,
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub source: UsageSource,
    pub refreshed_at_unix: Option<i64>,
    pub summary: Availability,
    pub limits: Vec<UsageLimit>,
    pub diagnostics: Vec<UsageDiagnostic>,
}

impl UsageSnapshot {
    pub fn unknown(source: UsageSource, diagnostic: UsageDiagnostic) -> Self {
        Self {
            source,
            refreshed_at_unix: None,
            summary: Availability::unknown(),
            limits: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageSource {
    RemoteApi,
    LocalSession,
    StoredSnapshot,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageLimit {
    pub id: String,
    pub label: String,
    pub scope: UsageLimitScope,
    pub kind: UsageLimitKind,
    pub window_seconds: Option<u64>,
    pub used_percent_x100: Option<u32>,
    pub remaining_percent_x100: Option<u32>,
    pub reset_at_unix: Option<i64>,
    pub exhausted: Option<bool>,
    pub raw_provider_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageLimitScope {
    Account,
    Workspace,
    Project,
    Model,
    Feature,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageLimitKind {
    RollingWindow,
    CalendarWindow,
    CreditBalance,
    RequestRate,
    TokenRate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UsageTokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_5m: Option<u64>,
    pub cache_write_1h: Option<u64>,
    pub reasoning: u64,
    pub extra: u64,
}

impl UsageTokenBreakdown {
    pub fn normalized_total(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
            .saturating_add(self.reasoning)
            .saturating_add(self.extra)
    }

    pub fn add(&mut self, other: &Self) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.cache_write_5m = add_optional_tokens(self.cache_write_5m, other.cache_write_5m);
        self.cache_write_1h = add_optional_tokens(self.cache_write_1h, other.cache_write_1h);
        self.reasoning = self.reasoning.saturating_add(other.reasoning);
        self.extra = self.extra.saturating_add(other.extra);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageDataQuality {
    Parsed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CostStatus {
    ProviderReported,
    Estimated,
    Missing,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageEventSource {
    pub kind: String,
    pub path: Option<PathBuf>,
    pub fingerprint_json: Option<String>,
    pub offset: Option<u64>,
    pub record_id: Option<String>,
    pub record_hash: Option<String>,
    pub backend: String,
    pub backend_version: String,
    pub parser_schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageFileSampleHash {
    pub offset: u64,
    pub len: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageRelatedSourceFingerprint {
    pub suffix: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified_unix_ns: u128,
    pub sample_hashes: Vec<UsageFileSampleHash>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSourceFingerprint {
    pub canonical_path: PathBuf,
    pub size: u64,
    pub modified_unix_ns: u128,
    pub sample_hashes: Vec<UsageFileSampleHash>,
    pub content_hash: String,
    pub related: Vec<UsageRelatedSourceFingerprint>,
    pub parser_schema_version: u32,
    pub backend_version: String,
}

impl UsageSourceFingerprint {
    pub fn from_path(
        path: &Path,
        parser_schema_version: u32,
        backend_version: impl Into<String>,
    ) -> crate::Result<Self> {
        Self::from_path_with_related(
            path,
            std::iter::empty::<(&str, PathBuf)>(),
            parser_schema_version,
            backend_version,
        )
    }

    pub fn from_path_with_related<I, S>(
        path: &Path,
        related: I,
        parser_schema_version: u32,
        backend_version: impl Into<String>,
    ) -> crate::Result<Self>
    where
        I: IntoIterator<Item = (S, PathBuf)>,
        S: Into<String>,
    {
        let canonical_path = path.canonicalize().map_err(|err| {
            crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
        })?;
        let file = fingerprint_file(&canonical_path)?;
        let mut related = related
            .into_iter()
            .filter_map(|(suffix, path)| {
                UsageRelatedSourceFingerprint::from_path(suffix.into(), &path).ok()
            })
            .collect::<Vec<_>>();
        related.sort_by(|left, right| left.suffix.cmp(&right.suffix));
        Ok(Self {
            canonical_path,
            size: file.size,
            modified_unix_ns: file.modified_unix_ns,
            sample_hashes: file.sample_hashes,
            content_hash: file.content_hash,
            related,
            parser_schema_version,
            backend_version: backend_version.into(),
        })
    }

    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string(self).map_err(|err| {
            crate::OpenMuxError::Message(format!("encode source fingerprint: {err}"))
        })
    }
}

impl UsageRelatedSourceFingerprint {
    fn from_path(suffix: String, path: &Path) -> crate::Result<Self> {
        let canonical_path = path.canonicalize().map_err(|err| {
            crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
        })?;
        let file = fingerprint_file(&canonical_path)?;
        Ok(Self {
            suffix,
            path: canonical_path,
            size: file.size,
            modified_unix_ns: file.modified_unix_ns,
            sample_hashes: file.sample_hashes,
            content_hash: file.content_hash,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageEvent {
    pub client: String,
    pub model_provider: Option<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub project_path: Option<PathBuf>,
    pub occurred_at_unix: i64,
    pub tokens: UsageTokenBreakdown,
    pub provider_total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub cost_status: CostStatus,
    pub source: UsageEventSource,
    pub quality: UsageDataQuality,
    pub event_hash: String,
}

impl UsageEvent {
    pub fn normalized_total_tokens(&self) -> u64 {
        self.tokens.normalized_total()
    }

    pub fn generated_event_hash(&self) -> String {
        let identity = if let Some(request_id) = self.request_id.as_deref() {
            serde_json::json!({
                "v": 1,
                "layer": "request_id",
                "client": self.client,
                "backend": self.source.backend,
                "parser_schema_version": self.source.parser_schema_version,
                "request_id": request_id,
            })
        } else if let Some(record_id) = self.source.record_id.as_deref() {
            serde_json::json!({
                "v": 1,
                "layer": "record_id",
                "client": self.client,
                "backend": self.source.backend,
                "parser_schema_version": self.source.parser_schema_version,
                "source_kind": self.source.kind,
                "source_path": self.source.path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                "record_id": record_id,
            })
        } else if let Some(offset) = self.source.offset {
            serde_json::json!({
                "v": 1,
                "layer": if self.source.record_hash.is_some() { "offset_record_hash" } else { "offset_fingerprint" },
                "client": self.client,
                "backend": self.source.backend,
                "parser_schema_version": self.source.parser_schema_version,
                "source_kind": self.source.kind,
                "source_path": self.source.path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                "offset": offset,
                "record_hash": self.source.record_hash,
                "fingerprint": self.source.fingerprint_json,
            })
        } else {
            serde_json::json!({
                "v": 1,
                "layer": "token_tuple",
                "client": self.client,
                "backend": self.source.backend,
                "parser_schema_version": self.source.parser_schema_version,
                "source_kind": self.source.kind,
                "source_path": self.source.path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                "session_id": self.session_id,
                "model_provider": self.model_provider,
                "model": self.model,
                "occurred_at_unix": self.occurred_at_unix,
                "tokens": self.tokens,
                "provider_total_tokens": self.provider_total_tokens,
            })
        };
        let bytes = serde_json::to_vec(&identity).unwrap_or_default();
        crate::storage::sha256_hex(&bytes)
    }

    pub fn set_generated_event_hash(&mut self) {
        self.event_hash = self.generated_event_hash();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageSummary {
    pub client: String,
    pub model_provider: Option<String>,
    pub model: Option<String>,
    pub project_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub tokens: UsageTokenBreakdown,
    pub normalized_total_tokens: u64,
    pub provider_total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub cost_status: CostStatus,
    pub quality: UsageDataQuality,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UsageSummaryQuery {
    pub client: Option<String>,
    pub since_unix: Option<i64>,
    pub until_unix: Option<i64>,
    pub model_provider: Option<String>,
    pub model: Option<String>,
    pub project_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub group_by_model_provider: bool,
    pub group_by_model: bool,
    pub group_by_project: bool,
    pub group_by_session: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageScanOptions {
    pub clients: Vec<String>,
    pub since_unix: Option<i64>,
    pub until_unix: Option<i64>,
    pub budget: UsageScanBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageScanBudget {
    pub timeout_ms: u64,
    pub max_source_files: usize,
    pub max_source_bytes: u64,
}

impl Default for UsageScanBudget {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            max_source_files: 2_000,
            max_source_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageScanReport {
    pub backend: String,
    pub backend_version: String,
    pub parser_schema_version: u32,
    pub events: Vec<UsageEvent>,
    pub diagnostics: Vec<UsageScanDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageScanDiagnostic {
    pub client: Option<String>,
    pub source_kind: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageScanWatermark {
    pub source_id: String,
    pub client: String,
    pub backend: String,
    pub backend_version: String,
    pub parser_schema_version: u32,
    pub source_kind: String,
    pub source_path: PathBuf,
    pub source_fingerprint: UsageSourceFingerprint,
    pub last_offset: Option<u64>,
    pub last_record_id: Option<String>,
    pub last_scanned_at_unix: u64,
    pub last_scan_status: String,
    pub diagnostic_code: Option<String>,
}

impl UsageScanWatermark {
    pub fn is_stale_for(&self, backend_version: &str, parser_schema_version: u32) -> bool {
        self.backend_version != backend_version
            || self.parser_schema_version != parser_schema_version
    }
}

struct FingerprintedFile {
    size: u64,
    modified_unix_ns: u128,
    sample_hashes: Vec<UsageFileSampleHash>,
    content_hash: String,
}

fn fingerprint_file(path: &Path) -> crate::Result<FingerprintedFile> {
    let metadata = fs::metadata(path).map_err(|err| {
        crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
    })?;
    let size = metadata.len();
    let modified_unix_ns = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(FingerprintedFile {
        size,
        modified_unix_ns,
        sample_hashes: sample_hashes(path, size)?,
        content_hash: file_hash(path)?,
    })
}

fn sample_hashes(path: &Path, size: u64) -> crate::Result<Vec<UsageFileSampleHash>> {
    if size == 0 {
        return Ok(Vec::new());
    }
    let sample_len = size.min(4096) as usize;
    let max_offset = size.saturating_sub(sample_len as u64);
    let mut offsets = if max_offset == 0 {
        vec![0]
    } else {
        vec![
            0,
            max_offset / 4,
            max_offset / 2,
            max_offset.saturating_mul(3) / 4,
            max_offset,
        ]
    };
    offsets.sort_unstable();
    offsets.dedup();

    let mut file = fs::File::open(path).map_err(|err| {
        crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
    })?;
    let mut hashes = Vec::with_capacity(offsets.len());
    for offset in offsets {
        file.seek(SeekFrom::Start(offset)).map_err(|err| {
            crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
        })?;
        let mut bytes = vec![0_u8; sample_len];
        let read = file.read(&mut bytes).map_err(|err| {
            crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
        })?;
        bytes.truncate(read);
        hashes.push(UsageFileSampleHash {
            offset,
            len: read as u64,
            hash: crate::storage::sha256_hex(&bytes),
        });
    }
    Ok(hashes)
}

fn file_hash(path: &Path) -> crate::Result<String> {
    let bytes = fs::read(path).map_err(|err| {
        crate::OpenMuxError::Message(format!("{}: {err}", path.to_string_lossy()))
    })?;
    Ok(crate::storage::sha256_hex(&bytes))
}

fn add_optional_tokens(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.saturating_add(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::unix_now_nanos;
    use std::{env, fs};

    #[test]
    fn token_breakdown_normalized_total_includes_all_buckets() {
        let tokens = UsageTokenBreakdown {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            cache_write_5m: Some(15),
            cache_write_1h: Some(25),
            reasoning: 50,
            extra: 60,
        };

        assert_eq!(tokens.normalized_total(), 210);
    }

    #[test]
    fn token_breakdown_add_preserves_optional_cache_breakdown() {
        let mut tokens = UsageTokenBreakdown {
            cache_write_5m: Some(5),
            ..UsageTokenBreakdown::default()
        };
        tokens.add(&UsageTokenBreakdown {
            cache_write_5m: Some(7),
            cache_write_1h: Some(11),
            input: 3,
            ..UsageTokenBreakdown::default()
        });

        assert_eq!(tokens.input, 3);
        assert_eq!(tokens.cache_write_5m, Some(12));
        assert_eq!(tokens.cache_write_1h, Some(11));
    }

    #[test]
    fn source_fingerprint_changes_when_file_content_changes() {
        let root = temp_test_dir("usage-fingerprint-content");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        fs::write(&path, b"{\"tokens\":1}\n").unwrap();
        let first = UsageSourceFingerprint::from_path(&path, 1, "backend-a").unwrap();

        fs::write(&path, b"{\"tokens\":2}\n").unwrap();
        let second = UsageSourceFingerprint::from_path(&path, 1, "backend-a").unwrap();

        assert_ne!(first.content_hash, second.content_hash);
        assert_ne!(first.sample_hashes, second.sample_hashes);
    }

    #[test]
    fn source_fingerprint_tracks_related_sidecar_changes() {
        let root = temp_test_dir("usage-fingerprint-sidecar");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        let sidecar = root.join("session.meta.json");
        fs::write(&path, b"{\"tokens\":1}\n").unwrap();
        fs::write(&sidecar, b"{\"agent\":\"a\"}\n").unwrap();
        let first = UsageSourceFingerprint::from_path_with_related(
            &path,
            [(".meta.json", sidecar.clone())],
            1,
            "backend-a",
        )
        .unwrap();

        fs::write(&sidecar, b"{\"agent\":\"b\"}\n").unwrap();
        let second = UsageSourceFingerprint::from_path_with_related(
            &path,
            [(".meta.json", sidecar)],
            1,
            "backend-a",
        )
        .unwrap();

        assert_eq!(first.content_hash, second.content_hash);
        assert_ne!(first.related, second.related);
    }

    #[test]
    fn scan_watermark_detects_backend_or_parser_version_staleness() {
        let root = temp_test_dir("usage-watermark-stale");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        fs::write(&path, b"{}\n").unwrap();
        let fingerprint = UsageSourceFingerprint::from_path(&path, 3, "backend-a").unwrap();
        let watermark = UsageScanWatermark {
            source_id: "source-1".to_string(),
            client: "codex".to_string(),
            backend: "tokscale".to_string(),
            backend_version: "backend-a".to_string(),
            parser_schema_version: 3,
            source_kind: "jsonl".to_string(),
            source_path: path,
            source_fingerprint: fingerprint,
            last_offset: Some(2),
            last_record_id: None,
            last_scanned_at_unix: 42,
            last_scan_status: "success".to_string(),
            diagnostic_code: None,
        };

        assert!(!watermark.is_stale_for("backend-a", 3));
        assert!(watermark.is_stale_for("backend-b", 3));
        assert!(watermark.is_stale_for("backend-a", 4));
    }

    #[test]
    fn event_hash_prefers_request_id_over_source_offsets() {
        let mut first = usage_event_for_hash();
        let mut second = first.clone();
        first.source.offset = Some(10);
        first.source.record_hash = Some("line-a".to_string());
        second.source.offset = Some(20);
        second.source.record_hash = Some("line-b".to_string());

        assert_eq!(first.generated_event_hash(), second.generated_event_hash());
    }

    #[test]
    fn event_hash_uses_offset_and_record_hash_when_request_id_is_missing() {
        let mut first = usage_event_for_hash();
        first.request_id = None;
        first.source.offset = Some(10);
        first.source.record_hash = Some("line-a".to_string());
        let mut second = first.clone();
        second.source.record_hash = Some("line-b".to_string());

        assert_ne!(first.generated_event_hash(), second.generated_event_hash());
    }

    #[test]
    fn event_hash_falls_back_to_token_tuple_when_source_identity_is_missing() {
        let mut first = usage_event_for_hash();
        first.request_id = None;
        first.source.offset = None;
        first.source.record_id = None;
        let mut second = first.clone();
        second.tokens.output = 999;

        assert_ne!(first.generated_event_hash(), second.generated_event_hash());
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("openmux-{name}-{}", unix_now_nanos()))
    }

    fn usage_event_for_hash() -> UsageEvent {
        UsageEvent {
            client: "codex".to_string(),
            model_provider: Some("openai".to_string()),
            model: Some("gpt-5".to_string()),
            session_id: Some("session-1".to_string()),
            request_id: Some("request-1".to_string()),
            project_path: Some(PathBuf::from("/tmp/project")),
            occurred_at_unix: 42,
            tokens: UsageTokenBreakdown {
                input: 10,
                output: 5,
                cache_read: 3,
                cache_write: 2,
                cache_write_5m: Some(2),
                cache_write_1h: None,
                reasoning: 7,
                extra: 1,
            },
            provider_total_tokens: Some(99),
            estimated_cost_usd: None,
            cost_status: CostStatus::Missing,
            source: UsageEventSource {
                kind: "jsonl".to_string(),
                path: Some(PathBuf::from("/tmp/session.jsonl")),
                fingerprint_json: Some(r#"{"size":123}"#.to_string()),
                offset: Some(12),
                record_id: None,
                record_hash: None,
                backend: "test-backend".to_string(),
                backend_version: "0.0.0".to_string(),
                parser_schema_version: 1,
            },
            quality: UsageDataQuality::Parsed,
            event_hash: String::new(),
        }
    }
}
