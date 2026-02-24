#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidProjectName,
    InvalidProjectNameFormat,
    ProjectNameTooLong,
}
