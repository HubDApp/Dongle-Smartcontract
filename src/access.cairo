use starknet::ContractAddress;
use starknet::get_caller_address;

/// Checks if the caller is the admin
pub fn only_admin(admin: ContractAddress) {
    let caller = get_caller_address();
    assert(caller == admin, 'ERR_NOT_ADMIN');
}

/// Checks if the caller is either the owner or admin
pub fn only_owner_or_admin(owner: ContractAddress, admin: ContractAddress) {
    let caller = get_caller_address();
    assert(caller == owner || caller == admin, 'ERR_NOT_OWNER_OR_ADMIN');
}
