#[derive(Debug)]
pub struct MmapError {
    pub code: i64,
    pub message: &'static str,
}