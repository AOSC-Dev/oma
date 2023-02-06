use rust_apt::{
    raw::progress::AcquireProgress,
    util::{time_str, unit_str, NumSys},
};

use crate::{msg, warn, success};

// TODO: Make better structs for pkgAcquire items, workers, owners.
/// AptAcquireProgress is the default struct for the update method on the cache.
///
/// This struct mimics the output of `apt update`.
#[derive(Default, Debug)]
pub struct NoProgress {
    _lastline: usize,
    _pulse_interval: usize,
    _disable: bool,
}

impl NoProgress {
    /// Returns a new default progress instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the AptAcquireProgress in a box
    /// To easily pass through for progress
    pub fn new_box() -> Box<dyn AcquireProgress> {
        Box::new(Self::new())
    }
}

/// Do not output anything apt AcquireProgress
impl AcquireProgress for NoProgress {
    fn pulse_interval(&self) -> usize {
        0
    }

    fn hit(&mut self, _id: u32, _description: String) {}

    fn fetch(&mut self, _id: u32, _description: String, _file_size: u64) {}

    fn fail(&mut self, _id: u32, _description: String, _status: u32, _error_text: String) {}

    fn pulse(
        &mut self,
        _workers: Vec<rust_apt::raw::progress::Worker>,
        _percent: f32,
        _total_bytes: u64,
        _current_bytes: u64,
        _current_cps: u64,
    ) {
    }

    fn done(&mut self) {}

    fn start(&mut self) {}

    fn stop(
        &mut self,
        fetched_bytes: u64,
        elapsed_time: u64,
        current_cps: u64,
        _pending_errors: bool,
    ) {
        if fetched_bytes != 0 {
            warn!("Download is not done, running apt download ...");
            println!(
                "Fetched {} in {} ({}/s)",
                unit_str(fetched_bytes, NumSys::Decimal),
                time_str(elapsed_time),
                unit_str(current_cps, NumSys::Decimal)
            );
        } else {
            success!("All Package is fetched");
        }
    }
}
