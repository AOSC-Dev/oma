use std::fmt;

use number_prefix::NumberPrefix;

// Formats bytes for human readability
#[derive(Debug)]
pub struct HumanBytes(pub u64);

impl fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match NumberPrefix::binary(self.0 as f64) {
            NumberPrefix::Standalone(number) => write!(f, "{number:.0}B"),
            NumberPrefix::Prefixed(prefix, number) => write!(f, "{number:.2} {prefix}B"),
        }
    }
}
