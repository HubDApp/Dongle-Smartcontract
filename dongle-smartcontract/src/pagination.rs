use crate::constants::MAX_PAGE_LIMIT;
use soroban_sdk::{Env, Vec};

pub fn paginate<T: Clone + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    items: &Vec<T>,
    start_index: u32,
    limit: u32,
) -> Vec<T>
where
    soroban_sdk::Val: soroban_sdk::TryFromVal<soroban_sdk::Env, T>,
{
    let limit = limit.min(MAX_PAGE_LIMIT);
    let total = items.len();
    let mut result = Vec::new(env);
    let mut count = 0u32;
    let mut i = start_index;
    while i < total && count < limit {
        if let Some(item) = items.get(i) {
            result.push_back(item);
            count += 1;
        }
        i += 1;
    }
    result
}

/// Opaque cursor token salt. Tokens must be treated as opaque by callers;
/// use `encode_cursor` / `decode_cursor` only via the pagination helpers.
const CURSOR_SALT: u64 = 0x9E37_79B9_7F4A_7C15;

/// Encode a result offset as an opaque cursor token for resumption.
pub fn encode_cursor(offset: u32) -> u64 {
    (offset as u64).wrapping_add(CURSOR_SALT).rotate_left(13)
}

/// Decode an opaque cursor token back to a result offset.
/// `None` (first page) and invalid tokens resolve to offset 0.
pub fn decode_cursor(cursor: Option<u64>) -> u32 {
    match cursor {
        None => 0,
        Some(token) => {
            let raw = token.rotate_right(13).wrapping_sub(CURSOR_SALT);
            if raw > u32::MAX as u64 {
                0
            } else {
                raw as u32
            }
        }
    }
}

/// Forward cursor pagination over an in-memory result set.
/// Returns `(page, next_cursor)`; `next_cursor` is `None` when exhausted.
pub fn paginate_with_cursor<T: Clone + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    items: &Vec<T>,
    cursor: Option<u64>,
    limit: u32,
) -> (Vec<T>, Option<u64>)
where
    soroban_sdk::Val: soroban_sdk::TryFromVal<soroban_sdk::Env, T>,
{
    let start = decode_cursor(cursor);
    let page = paginate(env, items, start, limit);
    let next_offset = start.saturating_add(page.len());
    let next = if next_offset < items.len() {
        Some(encode_cursor(next_offset))
    } else {
        None
    };
    (page, next)
}

/// Backward cursor pagination: `cursor` marks the exclusive end offset
/// (as produced by `encode_cursor`); page goes backwards from it.
pub fn paginate_backward<T: Clone + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    items: &Vec<T>,
    cursor: Option<u64>,
    limit: u32,
) -> (Vec<T>, Option<u64>)
where
    soroban_sdk::Val: soroban_sdk::TryFromVal<soroban_sdk::Env, T>,
{
    let limit = limit.min(MAX_PAGE_LIMIT);
    let total = items.len();
    let end = match cursor {
        None => total,
        Some(token) => decode_cursor(Some(token)).min(total),
    };
    let start = end.saturating_sub(limit);
    let mut result = Vec::new(env);
    let mut i = start;
    while i < end {
        if let Some(item) = items.get(i) {
            result.push_back(item);
        }
        i += 1;
    }
    let prev = if start > 0 {
        Some(encode_cursor(start))
    } else {
        None
    };
    (result, prev)
}
