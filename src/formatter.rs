use rust_apt::raw::progress::AcquireProgress;


// TODO: Make better structs for pkgAcquire items, workers, owners.
/// AptAcquireProgress is the default struct for the update method on the cache.
///
/// This struct mimics the output of `apt update`.
#[derive(Default, Debug)]
pub struct NoProgress {
	lastline: usize,
	pulse_interval: usize,
	disable: bool,
}

impl NoProgress {
	/// Returns a new default progress instance.
	pub fn new() -> Self { Self::default() }

	/// Return the AptAcquireProgress in a box
	/// To easily pass through for progress
	pub fn new_box() -> Box<dyn AcquireProgress> { Box::new(Self::new()) }
}

impl AcquireProgress for NoProgress {
    fn pulse_interval(&self) -> usize {
        0
    }

    fn hit(&mut self, id: u32, description: String) {
    }

    fn fetch(&mut self, id: u32, description: String, file_size: u64) {
    }

    fn fail(&mut self, id: u32, description: String, status: u32, error_text: String) {
    }

    fn pulse(
		&mut self,
		workers: Vec<rust_apt::raw::progress::Worker>,
		percent: f32,
		total_bytes: u64,
		current_bytes: u64,
		current_cps: u64,
	) {
    }

    fn done(&mut self) {
    }

    fn start(&mut self) {
    }

    fn stop(
		&mut self,
		fetched_bytes: u64,
		elapsed_time: u64,
		current_cps: u64,
		pending_errors: bool,
	) {
    }
}