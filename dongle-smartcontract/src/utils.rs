use soroban_sdk::{Env, String, Address, Bytes, BytesN, crypto};
use soroban_sdk::xdr::ToXdr;

pub fn compute_name_hash(env: &Env, name: &String) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.append(&name.clone().to_xdr(env));
    env.crypto().sha256(&data)
}

pub fn compute_metadata_hash(
    env: &Env,
    name: &String,
    description: &String,
    category: &String,
    owner: &Address,
) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.append(&name.clone().to_xdr(env));
    data.append(&description.clone().to_xdr(env));
    data.append(&category.clone().to_xdr(env));
    data.append(&owner.clone().to_xdr(env));
    
    env.crypto().sha256(&data)
}
