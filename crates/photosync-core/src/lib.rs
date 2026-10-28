//! Sync logic for photosync. UI-free by contract (INV-8): front-ends get
//! everything through this API, never through stdout or stderr.

mod source;

pub use source::{MediaType, ScanError, SourceFile, SourceScan, scan_source};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(VERSION, "0.1.0");
    }
}
