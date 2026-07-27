use soroban_sdk::{Env, Vec};

const MAX_PAGE_LIMIT: u32 = 100;

pub fn paginate<T: Clone>(env: &Env, items: &Vec<T>, start: u32, limit: u32) -> Vec<T> {
    let limit = limit.min(MAX_PAGE_LIMIT);
    let mut result = Vec::new(env);
    let mut count = 0u32;
    for (i, item) in items.iter().enumerate() {
        if (i as u32) < start {
            continue;
        }
        if count >= limit {
            break;
        }
        result.push_back(item);
        count += 1;
    }
    result
}
