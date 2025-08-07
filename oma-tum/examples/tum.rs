use std::path::Path;

use oma_pm_operation_type::{InstallEntry, InstallOperation, OmaOperation, PackageUrl};
use oma_tum::{get_matches_tum, get_tum};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tum = get_tum(&dir.join("examples/tum")).unwrap();
    let tum = get_matches_tum(
        &tum,
        &OmaOperation {
            install: vec![
                InstallEntry::builder()
                    .name("7-zip".to_string())
                    .name_without_arch("7-zip".to_string())
                    .new_version("25.10".to_string())
                    .new_size(0)
                    .pkg_urls(vec![PackageUrl {
                        download_url: Default::default(),
                        index_url: Default::default(),
                    }])
                    .arch("amd64".to_string())
                    .download_size(0)
                    .op(InstallOperation::Download)
                    .index(0)
                    .build(),
            ],
            remove: vec![],
            disk_size_delta: 0,
            autoremovable: (0, 0),
            total_download_size: 0,
            suggest: vec![],
            recommend: vec![],
        },
    );
    dbg!(tum);
}
