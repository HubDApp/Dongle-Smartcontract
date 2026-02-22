use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    EmptyProjectName = 1,
    InvalidProjectNameLength = 2,
    InvalidProjectNameFormat = 3,
}
