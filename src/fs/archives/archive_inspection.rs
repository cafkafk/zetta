#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveInspection {
    Always,
    Never,
    // TODO: option to limit file size (especially for compressed archives)
}
