use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ProcessData {
    pub pid: usize,
    pub name: String,
    #[cfg(not(target_os = "windows"))]
    pub uid: usize,
    #[cfg(target_os = "windows")]
    pub uid: String,
    pub username: String,
}
