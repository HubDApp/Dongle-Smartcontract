use core::array::Array;
use core::array::ArrayTrait;

/// Represents an IPFS CID as a single felt252 for simplicity
/// In production, this would be expanded to handle full CIDs
#[derive(Drop, Serde, starknet::Store, Copy)]
pub struct Cid {
    pub value: felt252,
}

/// Creates a Cid from a felt252 value
pub fn cid_from_parts(parts: Array<felt252>) -> Cid {
    // For now, just use the first part or 0 if empty
    let value = if parts.is_empty() { 0 } else { *parts.at(0) };
    Cid { value }
}

/// Returns the parts of a Cid as an array
pub fn cid_to_parts(ref cid: Cid) -> Array<felt252> {
    array![cid.value]
}

/// Creates an empty Cid
pub fn empty_cid() -> Cid {
    Cid { value: 0 }
}

/// Checks if a Cid is empty
pub fn is_empty_cid(ref cid: Cid) -> bool {
    cid.value == 0
}
