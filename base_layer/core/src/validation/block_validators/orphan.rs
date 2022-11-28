//  Copyright 2021, The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
use log::*;
use tari_utilities::hex::Hex;

use super::LOG_TARGET;
use crate::{
    blocks::Block,
    consensus::ConsensusManager,
    transactions::CryptoFactories,
    validation::{
        helpers::{
            check_accounting_balance,
            check_block_weight,
            check_coinbase_output,
            check_kernel_lock_height,
            check_maturity,
            check_output_features,
            check_permitted_output_types,
            check_sorting_and_duplicates,
            check_total_burned,
            check_validator_node_registration_utxo,
            validate_versions,
        },
        OrphanValidation,
        ValidationError,
    },
};

/// This validator tests whether a candidate block is internally consistent
#[derive(Clone)]
pub struct OrphanBlockValidator {
    rules: ConsensusManager,
    bypass_range_proof_verification: bool,
    factories: CryptoFactories,
}

impl OrphanBlockValidator {
    pub fn new(rules: ConsensusManager, bypass_range_proof_verification: bool, factories: CryptoFactories) -> Self {
        Self {
            rules,
            bypass_range_proof_verification,
            factories,
        }
    }
}

impl OrphanValidation for OrphanBlockValidator {
    /// The consensus checks that are done (in order of cheapest to verify to most expensive):
    /// 1. Is the block weight of the block under the prescribed limit?
    /// 1. Does it contain only unique inputs and outputs?
    /// 1. Where all the rules for the spent outputs followed?
    /// 1. Is there precisely one Coinbase output and is it correctly defined with the correct amount?
    /// 1. Is the accounting correct?
    fn validate(&self, block: &Block) -> Result<(), ValidationError> {
        let height = block.header.height;

        if height == 0 {
            warn!(target: LOG_TARGET, "Attempt to validate genesis block");
            return Err(ValidationError::ValidatingGenesis);
        }

        let block_id = if cfg!(debug_assertions) {
            format!(
                "block #{} {} ({})",
                height,
                block.hash().to_hex(),
                block.body.to_counts_string()
            )
        } else {
            format!("block #{}", height)
        };

        let constants = self.rules.consensus_constants(height);

        validate_versions(&block.body, constants)?;
        check_block_weight(block, constants)?;
        check_sorting_and_duplicates(&block.body)?;

        for output in block.body.outputs() {
            check_permitted_output_types(constants, output)?;
            check_validator_node_registration_utxo(constants, output)?;
        }

        check_total_burned(&block.body)?;

        // Check that the inputs are are allowed to be spent
        check_maturity(height, block.body.inputs())?;
        check_kernel_lock_height(height, block.body.kernels())?;
        check_output_features(block, &self.rules)?;
        check_coinbase_output(block, &self.rules, &self.factories)?;
        check_accounting_balance(
            block,
            &self.rules,
            self.bypass_range_proof_verification,
            &self.factories,
        )?;

        debug!(
            target: LOG_TARGET,
            "{} has PASSED stateless VALIDATION check.", &block_id
        );

        Ok(())
    }
}
