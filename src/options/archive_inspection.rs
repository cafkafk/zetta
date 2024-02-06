use crate::options::parser::MatchedFlags;
use crate::options::{flags, OptionsError};

#[derive(Debug, PartialEq)]
pub enum ArchiveInspection {
    Always,
    Never,
    // TODO: option to limit file size (especially for compressed archives)
}

impl ArchiveInspection {
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        Ok(if matches.has(&flags::INSPECT_ARCHIVES)? {
            ArchiveInspection::Always
        } else {
            ArchiveInspection::Never
        })
    }
}
