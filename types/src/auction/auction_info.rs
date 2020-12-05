// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    bytesrepr::{self, FromBytes, ToBytes},
    CLType, CLTyped, PublicKey, U512,
};

const SEIGNIORAGE_ALLOCATION_VALIDATOR_TAG: u8 = 0;
const SEIGNIORAGE_ALLOCATION_DELEGATOR_TAG: u8 = 1;

/// Information about a seigniorage allocation
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub enum SeigniorageAllocation {
    /// Info about a seigniorage allocation for a validator
    Validator {
        /// Validator's public key
        validator_public_key: PublicKey,
        /// Allocated amount
        amount: U512,
    },
    /// Info about a seigniorage allocation for a delegator
    Delegator {
        /// Delegator's public key
        delegator_public_key: PublicKey,
        /// Validator's public key
        validator_public_key: PublicKey,
        /// Allocated amount
        amount: U512,
    },
}

impl SeigniorageAllocation {
    /// Constructs a [`SeigniorageAllocation::Validator`]
    pub const fn validator(validator_public_key: PublicKey, amount: U512) -> Self {
        SeigniorageAllocation::Validator {
            validator_public_key,
            amount,
        }
    }

    /// Constructs a [`SeigniorageAllocation::Delegator`]
    pub const fn delegator(
        delegator_public_key: PublicKey,
        validator_public_key: PublicKey,
        amount: U512,
    ) -> Self {
        SeigniorageAllocation::Delegator {
            delegator_public_key,
            validator_public_key,
            amount,
        }
    }

    /// Returns the amount for a given seigniorage allocation
    pub fn amount(&self) -> &U512 {
        match self {
            SeigniorageAllocation::Validator { amount, .. } => amount,
            SeigniorageAllocation::Delegator { amount, .. } => amount,
        }
    }

    fn tag(&self) -> u8 {
        match self {
            SeigniorageAllocation::Validator { .. } => SEIGNIORAGE_ALLOCATION_VALIDATOR_TAG,
            SeigniorageAllocation::Delegator { .. } => SEIGNIORAGE_ALLOCATION_DELEGATOR_TAG,
        }
    }
}

impl ToBytes for SeigniorageAllocation {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        buffer.append(&mut self.tag().to_bytes()?);
        match self {
            SeigniorageAllocation::Validator {
                validator_public_key,
                amount,
            } => {
                buffer.append(&mut validator_public_key.to_bytes()?);
                buffer.append(&mut amount.to_bytes()?);
            }
            SeigniorageAllocation::Delegator {
                delegator_public_key,
                validator_public_key,
                amount,
            } => {
                buffer.append(&mut delegator_public_key.to_bytes()?);
                buffer.append(&mut validator_public_key.to_bytes()?);
                buffer.append(&mut amount.to_bytes()?);
            }
        }
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.tag().serialized_length()
            + match self {
                SeigniorageAllocation::Validator {
                    validator_public_key,
                    amount,
                } => validator_public_key.serialized_length() + amount.serialized_length(),
                SeigniorageAllocation::Delegator {
                    delegator_public_key,
                    validator_public_key,
                    amount,
                } => {
                    delegator_public_key.serialized_length()
                        + validator_public_key.serialized_length()
                        + amount.serialized_length()
                }
            }
    }
}

impl FromBytes for SeigniorageAllocation {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (tag, rem) = <u8>::from_bytes(bytes)?;
        match tag {
            SEIGNIORAGE_ALLOCATION_VALIDATOR_TAG => {
                let (validator_public_key, rem) = PublicKey::from_bytes(rem)?;
                let (amount, rem) = U512::from_bytes(rem)?;
                Ok((
                    SeigniorageAllocation::validator(validator_public_key, amount),
                    rem,
                ))
            }
            SEIGNIORAGE_ALLOCATION_DELEGATOR_TAG => {
                let (delegator_public_key, rem) = PublicKey::from_bytes(rem)?;
                let (validator_public_key, rem) = PublicKey::from_bytes(rem)?;
                let (amount, rem) = U512::from_bytes(rem)?;
                Ok((
                    SeigniorageAllocation::delegator(
                        delegator_public_key,
                        validator_public_key,
                        amount,
                    ),
                    rem,
                ))
            }
            _ => Err(bytesrepr::Error::Formatting),
        }
    }
}

impl CLTyped for SeigniorageAllocation {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

/// Auction metdata.  Intended to be recorded at each era.
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AuctionInfo {
    seigniorage_allocations: Vec<SeigniorageAllocation>,
}

impl AuctionInfo {
    /// Constructs a [`AuctionInfo`].
    pub fn new() -> Self {
        let seigniorage_allocations = Vec::new();
        AuctionInfo {
            seigniorage_allocations,
        }
    }

    /// Returns a reference to the seigniorage allocations collection
    pub fn seigniorage_allocations(&self) -> &Vec<SeigniorageAllocation> {
        &self.seigniorage_allocations
    }

    /// Returns a mutable reference to the seigniorage allocations collection
    pub fn seigniorage_allocations_mut(&mut self) -> &mut Vec<SeigniorageAllocation> {
        &mut self.seigniorage_allocations
    }

    /// Returns all seigniorage allocations that match the provided public key
    /// using the following criteria:
    /// * If the match candidate is a validator allocation, the provided public key is matched
    ///   against the validator public key.
    /// * If the match candidate is a delegator allocation, the provided public key is matched
    ///   against the delegator public key.
    pub fn select(&self, public_key: PublicKey) -> impl Iterator<Item = &SeigniorageAllocation> {
        self.seigniorage_allocations
            .iter()
            .filter(move |allocation| match allocation {
                SeigniorageAllocation::Validator {
                    validator_public_key,
                    ..
                } => public_key == *validator_public_key,
                SeigniorageAllocation::Delegator {
                    delegator_public_key,
                    ..
                } => public_key == *delegator_public_key,
            })
    }
}

impl ToBytes for AuctionInfo {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        self.seigniorage_allocations.to_bytes()
    }

    fn serialized_length(&self) -> usize {
        self.seigniorage_allocations.serialized_length()
    }
}

impl FromBytes for AuctionInfo {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (seigniorage_allocations, rem) = Vec::<SeigniorageAllocation>::from_bytes(bytes)?;
        Ok((
            AuctionInfo {
                seigniorage_allocations,
            },
            rem,
        ))
    }
}

impl CLTyped for AuctionInfo {
    fn cl_type() -> CLType {
        CLType::List(Box::new(SeigniorageAllocation::cl_type()))
    }
}

#[cfg(test)]
pub(crate) mod gens {
    use proptest::{
        collection::{self, SizeRange},
        prelude::Strategy,
        prop_oneof,
    };

    use crate::{
        auction::{AuctionInfo, SeigniorageAllocation},
        gens::u512_arb,
        public_key::gens::public_key_arb,
    };

    fn seigniorage_allocation_validator_arb() -> impl Strategy<Value = SeigniorageAllocation> {
        (public_key_arb(), u512_arb()).prop_map(|(validator_public_key, amount)| {
            SeigniorageAllocation::validator(validator_public_key, amount)
        })
    }

    fn seigniorage_allocation_delegator_arb() -> impl Strategy<Value = SeigniorageAllocation> {
        (public_key_arb(), public_key_arb(), u512_arb()).prop_map(
            |(delegator_public_key, validator_public_key, amount)| {
                SeigniorageAllocation::delegator(delegator_public_key, validator_public_key, amount)
            },
        )
    }

    pub fn seigniorage_allocation_arb() -> impl Strategy<Value = SeigniorageAllocation> {
        prop_oneof![
            seigniorage_allocation_validator_arb(),
            seigniorage_allocation_delegator_arb()
        ]
    }

    pub fn auction_info_arb(size: impl Into<SizeRange>) -> impl Strategy<Value = AuctionInfo> {
        collection::vec(seigniorage_allocation_arb(), size).prop_map(|allocations| {
            let mut auction_info = AuctionInfo::new();
            *auction_info.seigniorage_allocations_mut() = allocations;
            auction_info
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::bytesrepr;

    use super::gens;

    proptest! {
        #[test]
        fn test_serialization_roundtrip(auction_info in gens::auction_info_arb(0..32)) {
            bytesrepr::test_serialization_roundtrip(&auction_info)
        }
    }
}