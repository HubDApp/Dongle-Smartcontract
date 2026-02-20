use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    DuplicateProject = 1,
    DuplicateName = 2,
    Unauthorized = 3,
    NotFound = 4,
}
