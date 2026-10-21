//! Dependency version requirement parsing and matching (#923).
//!
//! Supported spec formats:
//! - `1.2.3`  — exact version (all three components required)
//! - `^1.2`   — caret: compatible with the leftmost non-zero component
//! - `~1.2.3` — tilde: patch-level changes (minor-level if only major given)
//! - `>=1.0`  — any version at or above the given one
//!
//! Candidate versions are `MAJOR.MINOR.PATCH`, optionally prefixed with `v`.
//! Any pre-release / build suffix (`-rc.1`, `+build`) is ignored.

use crate::errors::ContractError;
use crate::types::{VersionOperator, VersionSpec};
use soroban_sdk::String;

/// Maximum length in bytes of a version spec or candidate version string.
pub const MAX_VERSION_STR_LEN: usize = 32;

type Triple = (u32, u32, u32);

pub struct VersionSpecParser;

impl VersionSpecParser {
    fn to_bytes(s: &String, buf: &mut [u8; MAX_VERSION_STR_LEN]) -> Option<usize> {
        let len = s.len() as usize;
        if len == 0 || len > MAX_VERSION_STR_LEN {
            return None;
        }
        s.copy_into_slice(&mut buf[..len]);
        Some(len)
    }

    /// Parse a non-empty decimal component without leading zeros.
    fn parse_component(bytes: &[u8]) -> Option<u32> {
        if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == b'0') {
            return None;
        }
        let mut value: u32 = 0;
        for &b in bytes {
            if !b.is_ascii_digit() {
                return None;
            }
            value = value.checked_mul(10)?.checked_add((b - b'0') as u32)?;
        }
        Some(value)
    }

    /// Parse 1–3 dot-separated components.
    fn parse_components(bytes: &[u8]) -> Option<(u32, Option<u32>, Option<u32>)> {
        let mut parts: [Option<u32>; 3] = [None, None, None];
        let mut count = 0;
        for part in bytes.split(|&b| b == b'.') {
            if count == 3 {
                return None;
            }
            parts[count] = Some(Self::parse_component(part)?);
            count += 1;
        }
        Some((parts[0]?, parts[1], parts[2]))
    }

    /// Parse and validate a version requirement string.
    pub fn parse(raw: &String) -> Result<VersionSpec, ContractError> {
        let mut buf = [0u8; MAX_VERSION_STR_LEN];
        let len = Self::to_bytes(raw, &mut buf).ok_or(ContractError::InvalidVersionSpec)?;
        let bytes = &buf[..len];

        let (operator, rest) = if let Some(r) = bytes.strip_prefix(b">=") {
            (VersionOperator::GreaterOrEqual, r)
        } else if let Some(r) = bytes.strip_prefix(b"^") {
            (VersionOperator::Caret, r)
        } else if let Some(r) = bytes.strip_prefix(b"~") {
            (VersionOperator::Tilde, r)
        } else {
            (VersionOperator::Exact, bytes)
        };

        let (major, minor, patch) =
            Self::parse_components(rest).ok_or(ContractError::InvalidVersionSpec)?;
        if operator == VersionOperator::Exact && patch.is_none() {
            return Err(ContractError::InvalidVersionSpec);
        }

        Ok(VersionSpec {
            raw: raw.clone(),
            operator,
            major,
            minor,
            patch,
        })
    }

    /// Parse a concrete `MAJOR.MINOR.PATCH` version, tolerating a leading `v`
    /// and ignoring any pre-release / build suffix.
    pub fn parse_version(version: &String) -> Option<Triple> {
        let mut buf = [0u8; MAX_VERSION_STR_LEN];
        let len = Self::to_bytes(version, &mut buf)?;
        let mut bytes = &buf[..len];
        if let Some(r) = bytes.strip_prefix(b"v") {
            bytes = r;
        }
        if let Some(end) = bytes.iter().position(|&b| b == b'-' || b == b'+') {
            bytes = &bytes[..end];
        }
        match Self::parse_components(bytes)? {
            (major, Some(minor), Some(patch)) => Some((major, minor, patch)),
            _ => None,
        }
    }

    /// Whether `version` satisfies `spec`.
    pub fn matches(spec: &VersionSpec, version: Triple) -> bool {
        let minor = spec.minor.unwrap_or(0);
        let patch = spec.patch.unwrap_or(0);
        let lower = (spec.major, minor, patch);

        match spec.operator {
            VersionOperator::Exact => version == lower,
            VersionOperator::GreaterOrEqual => version >= lower,
            VersionOperator::Caret => {
                let upper = if spec.major > 0 || spec.minor.is_none() {
                    (spec.major.saturating_add(1), 0, 0)
                } else if minor > 0 || spec.patch.is_none() {
                    (0, minor.saturating_add(1), 0)
                } else {
                    (0, 0, patch.saturating_add(1))
                };
                version >= lower && version < upper
            }
            VersionOperator::Tilde => {
                let upper = if spec.minor.is_some() {
                    (spec.major, minor.saturating_add(1), 0)
                } else {
                    (spec.major.saturating_add(1), 0, 0)
                };
                version >= lower && version < upper
            }
        }
    }
}
