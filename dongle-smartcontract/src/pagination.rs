use crate::constants::MAX_PAGE_LIMIT;
use soroban_sdk::{Env, Vec};

pub fn paginate<T: Clone + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    items: &Vec<T>,
    start: u32,
    limit: u32,
) -> Vec<T>
where
    soroban_sdk::Val: soroban_sdk::TryFromVal<soroban_sdk::Env, T>,
{
    let limit = limit.min(MAX_PAGE_LIMIT);
    let total = items.len();
    let mut result = Vec::new(env);
    let mut count = 0u32;
    let mut i = start;
    while i < total && count < limit {
        if let Some(item) = items.get(i) {
            result.push_back(item);
            count += 1;
        }
        i += 1;
    }
    result
}
