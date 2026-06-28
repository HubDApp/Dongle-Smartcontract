use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // Existing errors (assume they are present, we only add new ones)
    // ...
    InvalidBountyUrl = 100,
    InvalidBountyCid = 101,
}
