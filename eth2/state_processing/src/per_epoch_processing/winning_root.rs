use std::collections::HashSet;
use std::iter::FromIterator;
use types::*;

#[derive(Clone)]
pub struct WinningRoot {
    pub crosslink_data_root: Hash256,
    pub attesting_validator_indices: Vec<usize>,
    pub total_attesting_balance: u64,
}

impl WinningRoot {
    /// Returns `true` if `self` is a "better" candidate than `other`.
    ///
    /// A winning root is "better" than another if it has a higher `total_attesting_balance`. Ties
    /// are broken by favouring the lower `crosslink_data_root` value.
    ///
    /// Spec v0.4.0
    pub fn is_better_than(&self, other: &Self) -> bool {
        if self.total_attesting_balance > other.total_attesting_balance {
            true
        } else if self.total_attesting_balance == other.total_attesting_balance {
            self.crosslink_data_root < other.crosslink_data_root
        } else {
            false
        }
    }
}

/// Returns the `crosslink_data_root` with the highest total attesting balance for the given shard.
/// Breaks ties by favouring the smaller `crosslink_data_root` hash.
///
/// The `WinningRoot` object also contains additional fields that are useful in later stages of
/// per-epoch processing.
///
/// Spec v0.4.0
pub fn winning_root(
    state: &BeaconState,
    shard: u64,
    current_epoch_attestations: &[&PendingAttestation],
    previous_epoch_attestations: &[&PendingAttestation],
    spec: &ChainSpec,
) -> Result<Option<WinningRoot>, BeaconStateError> {
    let mut winning_root: Option<WinningRoot> = None;

    let crosslink_data_roots: HashSet<Hash256> = HashSet::from_iter(
        previous_epoch_attestations
            .iter()
            .chain(current_epoch_attestations.iter())
            .filter_map(|a| {
                if a.data.shard == shard {
                    Some(a.data.crosslink_data_root)
                } else {
                    None
                }
            }),
    );

    for crosslink_data_root in crosslink_data_roots {
        let attesting_validator_indices = get_attesting_validator_indices(
            state,
            shard,
            current_epoch_attestations,
            previous_epoch_attestations,
            &crosslink_data_root,
            spec,
        )?;

        let total_attesting_balance: u64 = attesting_validator_indices
            .iter()
            .fold(0, |acc, i| acc + state.get_effective_balance(*i, spec));

        let candidate = WinningRoot {
            crosslink_data_root,
            attesting_validator_indices,
            total_attesting_balance,
        };

        if let Some(ref winner) = winning_root {
            if candidate.is_better_than(&winner) {
                winning_root = Some(candidate);
            }
        } else {
            winning_root = Some(candidate);
        }
    }

    Ok(winning_root)
}

/// Returns all indices which voted for a given crosslink. May contain duplicates.
///
/// Spec v0.4.0
fn get_attesting_validator_indices(
    state: &BeaconState,
    shard: u64,
    current_epoch_attestations: &[&PendingAttestation],
    previous_epoch_attestations: &[&PendingAttestation],
    crosslink_data_root: &Hash256,
    spec: &ChainSpec,
) -> Result<Vec<usize>, BeaconStateError> {
    let mut indices = vec![];

    for a in current_epoch_attestations
        .iter()
        .chain(previous_epoch_attestations.iter())
    {
        if (a.data.shard == shard) && (a.data.crosslink_data_root == *crosslink_data_root) {
            indices.append(&mut state.get_attestation_participants(
                &a.data,
                &a.aggregation_bitfield,
                spec,
            )?);
        }
    }

    Ok(indices)
}