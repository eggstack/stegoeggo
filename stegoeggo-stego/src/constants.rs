pub(crate) const STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95;

pub(crate) const STEGO_SPREAD_FACTOR: usize = 5;

pub(crate) const SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15;

pub(crate) const MIN_REDUNDANCY: usize = 1;

pub(crate) const MAX_REDUNDANCY: usize = 10;

pub(crate) fn validate_redundancy(redundancy: usize) -> Result<(), super::StegoError> {
    if (MIN_REDUNDANCY..=MAX_REDUNDANCY).contains(&redundancy) {
        Ok(())
    } else {
        Err(super::StegoError::InvalidConfig(format!(
            "redundancy must be in {MIN_REDUNDANCY}..={MAX_REDUNDANCY}, got {redundancy}"
        )))
    }
}
