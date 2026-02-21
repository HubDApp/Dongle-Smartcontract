use soroban_sdk::{Env, Address};
use crate::types::DataKey;

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn check_admin(env: &Env, caller: &Address) {
    if let Some(admin) = get_admin(env) {
        if &admin != caller {
            panic!("not authorized");
        }
    } else {
        panic!("admin not set");
    }
}
