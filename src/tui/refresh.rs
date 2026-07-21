use oma_fetch::Event as FetchEvent;

/// A single active download shown in the loading screen.
pub struct DownloadItem {
    pub index: usize,
    pub total: usize,
    pub msg: String,
    pub size: u64,
    pub downloaded: u64,
}

pub fn update_download_items(items: &mut Vec<DownloadItem>, event: FetchEvent) {
    match event {
        FetchEvent::NewProgressBar {
            index,
            total,
            msg,
            size,
            ..
        } => {
            if let Some(item) = items.iter_mut().find(|i| i.index == index) {
                item.size = size;
                item.msg = msg;
                item.total = total;
            } else {
                items.push(DownloadItem {
                    index,
                    total,
                    msg,
                    size,
                    downloaded: 0,
                });
            }
        }
        FetchEvent::NewProgressSpinner {
            index, total, msg, ..
        } => {
            if !items.iter().any(|i| i.index == index) {
                items.push(DownloadItem {
                    index,
                    total,
                    msg,
                    size: 0,
                    downloaded: 0,
                });
            }
        }
        FetchEvent::ProgressInc { index, size } => {
            if let Some(item) = items.iter_mut().find(|i| i.index == index) {
                item.downloaded = item.downloaded.saturating_add(size);
            }
        }
        FetchEvent::ProgressDone(index) => {
            if let Some(item) = items.iter_mut().find(|i| i.index == index) {
                item.downloaded = item.size;
            }
        }
        _ => {}
    }
}
