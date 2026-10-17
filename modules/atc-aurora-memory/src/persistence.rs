// Copyright (c) 2026 A-TownChain-Okosystems. All Rights Reserved.
// Durable, versioned journal for Aurora memory records.
//
// Payloads are opaque bytes: callers must encode/decode their domain record and must
// not assume this journal provides encryption-at-rest. Scope checks are enforced here
// in addition to any authorization at the application boundary.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MAGIC_V1: &[u8; 8] = b"ATCMEM01";
const MAGIC_V2: &[u8; 8] = b"ATCMEM02";
const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;
const MAX_FIELD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Io(String),
    Corrupt(String),
    InvalidInput(&'static str),
    AccessDenied,
    NotFound,
}

impl From<io::Error> for StoreError {
    fn from(value: io::Error) -> Self { Self::Io(value.to_string()) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableRecord {
    pub id: u64,
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub approved: bool,
    pub payload: Vec<u8>,
}

impl DurableRecord {
    fn validate(&self) -> Result<(), StoreError> {
        if self.tenant_id.trim().is_empty() || self.user_id.trim().is_empty()
            || self.agent_id.trim().is_empty() {
            return Err(StoreError::InvalidInput("scope identifiers must be non-empty"));
        }
        for field in [&self.tenant_id, &self.user_id, &self.agent_id] {
            if field.len() > MAX_FIELD_BYTES {
                return Err(StoreError::InvalidInput("scope field exceeds limit"));
            }
        }
        if self.payload.len() > MAX_RECORD_BYTES {
            return Err(StoreError::InvalidInput("payload exceeds record limit"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePrincipal {
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
}

/// Append-only persistence with fail-closed parsing and atomic compaction on deletion.
/// Schema v1 -> v2 migration changes the header only because v1 used the same record
/// encoding. Unknown versions and truncated records are rejected rather than ignored.
pub struct DurableMemoryStore {
    path: PathBuf,
    records: Vec<DurableRecord>,
}

impl DurableMemoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
            file.write_all(MAGIC_V2)?;
            file.sync_all()?;
            return Ok(Self { path, records: Vec::new() });
        }

        let mut bytes = Vec::new();
        File::open(&path)?.read_to_end(&mut bytes)?;
        if bytes.len() < 8 {
            return Err(StoreError::Corrupt("missing schema header".into()));
        }
        let legacy = &bytes[..8] == MAGIC_V1;
        if !legacy && &bytes[..8] != MAGIC_V2 {
            return Err(StoreError::Corrupt("unsupported schema version".into()));
        }
        let records = decode_records(&bytes[8..])?;
        let store = Self { path, records };
        if legacy { store.rewrite_with_header(MAGIC_V2)?; }
        Ok(store)
    }

    pub fn schema_version(&self) -> u8 { 2 }

    /// Internal recovery view. Application APIs must use scoped get/list instead.
    pub(crate) fn records_for_recovery(&self) -> &[DurableRecord] { &self.records }

    pub fn put(&mut self, record: DurableRecord) -> Result<(), StoreError> {
        record.validate()?;
        // IDs are canonical keys; only update the in-memory snapshot after durable append.
        let encoded = encode_record(&record)?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(&(encoded.len() as u32).to_be_bytes())?;
        file.write_all(&encoded)?;
        file.sync_all()?;
        self.records.retain(|existing| existing.id != record.id);
        self.records.push(record);
        Ok(())
    }

    pub fn get(&self, principal: &StorePrincipal, id: u64, now: u64)
        -> Result<DurableRecord, StoreError> {
        let record = self.records.iter().find(|r| r.id == id).ok_or(StoreError::NotFound)?;
        if record.tenant_id != principal.tenant_id || record.user_id != principal.user_id
            || record.agent_id != principal.agent_id {
            return Err(StoreError::AccessDenied);
        }
        if !record.approved || record.expires_at.map_or(false, |expiry| expiry <= now) {
            return Err(StoreError::NotFound);
        }
        Ok(record.clone())
    }

    pub fn list(&self, principal: &StorePrincipal, now: u64) -> Vec<DurableRecord> {
        self.records.iter().filter(|r| {
            r.tenant_id == principal.tenant_id && r.user_id == principal.user_id
                && r.agent_id == principal.agent_id && r.approved
                && r.expires_at.map_or(true, |expiry| expiry > now)
        }).cloned().collect()
    }

    /// Delete is idempotent. The replacement file is synced before it replaces the
    /// canonical journal; on failure, the in-memory snapshot is not changed.
    pub fn delete(&mut self, principal: &StorePrincipal, id: u64) -> Result<(), StoreError> {
        let record = self.records.iter().find(|r| r.id == id).ok_or(StoreError::NotFound)?;
        if record.tenant_id != principal.tenant_id || record.user_id != principal.user_id
            || record.agent_id != principal.agent_id {
            return Err(StoreError::AccessDenied);
        }
        let retained: Vec<_> = self.records.iter().filter(|r| r.id != id).cloned().collect();
        self.rewrite_records(&retained)?;
        self.records = retained;
        Ok(())
    }

    fn rewrite_with_header(&self, header: &[u8; 8]) -> Result<(), StoreError> {
        self.rewrite_to(&self.records, header)
    }

    fn rewrite_records(&self, records: &[DurableRecord]) -> Result<(), StoreError> {
        self.rewrite_to(records, MAGIC_V2)
    }

    fn rewrite_to(&self, records: &[DurableRecord], header: &[u8; 8]) -> Result<(), StoreError> {
        let temp = self.path.with_extension("journal.tmp");
        {
            let mut file = File::create(&temp)?;
            file.write_all(header)?;
            for record in records {
                let encoded = encode_record(record)?;
                file.write_all(&(encoded.len() as u32).to_be_bytes())?;
                file.write_all(&encoded)?;
            }
            file.sync_all()?;
        }
        fs::rename(&temp, &self.path)?;
        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = File::open(parent) { let _ = dir.sync_all(); }
        }
        Ok(())
    }
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), StoreError> {
    if bytes.len() > MAX_RECORD_BYTES || bytes.len() > u32::MAX as usize {
        return Err(StoreError::InvalidInput("field exceeds encoding limit"));
    }
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn encode_record(record: &DurableRecord) -> Result<Vec<u8>, StoreError> {
    record.validate()?;
    let mut out = Vec::new();
    out.extend_from_slice(&record.id.to_be_bytes());
    out.extend_from_slice(&record.created_at.to_be_bytes());
    out.extend_from_slice(&record.expires_at.unwrap_or(u64::MAX).to_be_bytes());
    out.push(u8::from(record.approved));
    push_bytes(&mut out, record.tenant_id.as_bytes())?;
    push_bytes(&mut out, record.user_id.as_bytes())?;
    push_bytes(&mut out, record.agent_id.as_bytes())?;
    push_bytes(&mut out, &record.payload)?;
    if out.len() > MAX_RECORD_BYTES { return Err(StoreError::InvalidInput("record too large")); }
    Ok(out)
}

fn take<'a>(bytes: &'a [u8], offset: &mut usize, count: usize) -> Result<&'a [u8], StoreError> {
    let end = offset.checked_add(count).ok_or_else(|| StoreError::Corrupt("length overflow".into()))?;
    if end > bytes.len() { return Err(StoreError::Corrupt("truncated record".into())); }
    let result = &bytes[*offset..end];
    *offset = end;
    Ok(result)
}

fn take_u32(bytes: &[u8], offset: &mut usize) -> Result<usize, StoreError> {
    let raw = take(bytes, offset, 4)?;
    Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize)
}

fn take_string(bytes: &[u8], offset: &mut usize) -> Result<String, StoreError> {
    let len = take_u32(bytes, offset)?;
    if len > MAX_FIELD_BYTES { return Err(StoreError::Corrupt("scope field too large".into())); }
    String::from_utf8(take(bytes, offset, len)?.to_vec())
        .map_err(|_| StoreError::Corrupt("scope field is not UTF-8".into()))
}

fn decode_record(bytes: &[u8]) -> Result<DurableRecord, StoreError> {
    let mut offset = 0;
    let idb = take(bytes, &mut offset, 8)?;
    let createdb = take(bytes, &mut offset, 8)?;
    let expiryb = take(bytes, &mut offset, 8)?;
    let approved = *take(bytes, &mut offset, 1)?.first().unwrap();
    if approved > 1 { return Err(StoreError::Corrupt("invalid approval flag".into())); }
    let id = u64::from_be_bytes(idb.try_into().unwrap());
    let created_at = u64::from_be_bytes(createdb.try_into().unwrap());
    let expiry = u64::from_be_bytes(expiryb.try_into().unwrap());
    let tenant_id = take_string(bytes, &mut offset)?;
    let user_id = take_string(bytes, &mut offset)?;
    let agent_id = take_string(bytes, &mut offset)?;
    let payload_len = take_u32(bytes, &mut offset)?;
    if payload_len > MAX_RECORD_BYTES { return Err(StoreError::Corrupt("payload too large".into())); }
    let payload = take(bytes, &mut offset, payload_len)?.to_vec();
    if offset != bytes.len() { return Err(StoreError::Corrupt("trailing bytes in record".into())); }
    let record = DurableRecord {
        id, tenant_id, user_id, agent_id, created_at,
        expires_at: if expiry == u64::MAX { None } else { Some(expiry) },
        approved: approved == 1, payload,
    };
    record.validate()?;
    Ok(record)
}

fn decode_records(mut bytes: &[u8]) -> Result<Vec<DurableRecord>, StoreError> {
    let mut records = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 4 { return Err(StoreError::Corrupt("truncated record length".into())); }
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        bytes = &bytes[4..];
        if len > MAX_RECORD_BYTES { return Err(StoreError::Corrupt("record exceeds limit".into())); }
        if bytes.len() < len { return Err(StoreError::Corrupt("truncated record payload".into())); }
        let (record_bytes, rest) = bytes.split_at(len);
        let record = decode_record(record_bytes)?;
        records.retain(|existing: &DurableRecord| existing.id != record.id);
        records.push(record);
        bytes = rest;
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn path(label: &str) -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("aurora-memory-{label}-{}-{n}.db", std::process::id()))
    }
    fn principal() -> StorePrincipal {
        StorePrincipal { tenant_id: "t1".into(), user_id: "u1".into(), agent_id: "a1".into() }
    }
    fn record(id: u64) -> DurableRecord {
        DurableRecord { id, tenant_id: "t1".into(), user_id: "u1".into(), agent_id: "a1".into(),
            created_at: 10, expires_at: None, approved: true, payload: b"remembered".to_vec() }
    }

    #[test]
    fn persists_across_reopen_and_enforces_scope() {
        let p = path("restart");
        { let mut store = DurableMemoryStore::open(&p).unwrap(); store.put(record(7)).unwrap(); }
        let store = DurableMemoryStore::open(&p).unwrap();
        assert_eq!(store.get(&principal(), 7, 11).unwrap().payload, b"remembered");
        let other = StorePrincipal { user_id: "other".into(), ..principal() };
        assert_eq!(store.get(&other, 7, 11), Err(StoreError::AccessDenied));
        let _ = fs::remove_file(p);
    }

    #[test]
    fn filters_unapproved_and_expired_records() {
        let p = path("filter");
        let mut store = DurableMemoryStore::open(&p).unwrap();
        let mut unapproved = record(1); unapproved.approved = false;
        let mut expired = record(2); expired.expires_at = Some(20);
        store.put(unapproved).unwrap(); store.put(expired).unwrap();
        assert!(store.list(&principal(), 20).is_empty());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn delete_survives_restart() {
        let p = path("delete");
        { let mut store = DurableMemoryStore::open(&p).unwrap(); store.put(record(9)).unwrap();
          store.delete(&principal(), 9).unwrap(); }
        let store = DurableMemoryStore::open(&p).unwrap();
        assert_eq!(store.get(&principal(), 9, 11), Err(StoreError::NotFound));
        let _ = fs::remove_file(p);
    }

    #[test]
    fn migrates_v1_header_and_rejects_truncation() {
        let p = path("migration");
        let encoded = encode_record(&record(4)).unwrap();
        { let mut file = File::create(&p).unwrap(); file.write_all(MAGIC_V1).unwrap();
          file.write_all(&(encoded.len() as u32).to_be_bytes()).unwrap(); file.write_all(&encoded).unwrap(); }
        let store = DurableMemoryStore::open(&p).unwrap();
        assert_eq!(store.schema_version(), 2);
        assert_eq!(&fs::read(&p).unwrap()[..8], MAGIC_V2);
        let _ = fs::remove_file(&p);

        let bad = path("truncated");
        { let mut file = File::create(&bad).unwrap(); file.write_all(MAGIC_V2).unwrap();
          file.write_all(&[0, 0, 0, 9, 1]).unwrap(); }
        assert!(matches!(DurableMemoryStore::open(&bad), Err(StoreError::Corrupt(_))));
        let _ = fs::remove_file(bad);
    }
}
