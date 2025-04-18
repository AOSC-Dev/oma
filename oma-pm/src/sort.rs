#[derive(Debug, Default)]
pub struct SummarySort {
    pub(crate) names: bool,
    pub(crate) operation: bool,
}

impl SummarySort {
    pub fn names(mut self) -> Self {
        self.names = true;
        self
    }

    pub fn operation(mut self) -> Self {
        self.operation = true;
        self
    }
}
