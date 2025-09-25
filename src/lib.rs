//! SolSVM. A simple SPL payment channel.
//!
//! SolSVM is an SVM-based payment channel that allows two parties to exchange
//! tokens off-chain. The channel is opened by invoking the SolSVM "VM",
//! running on some arbitrary server(s). When transacting has concluded, the
//! channel is closed by submitting the final payment ledger to Solana.
//!
//! The final ledger tracks debits and credits to all registered token accounts
//! or system accounts (native SOL) during the lifetime of a channel. It is
//! then used to to craft a batch of transactions to submit to the settlement
//! chain (Solana).
//!
//! Users opt-in to using a SolSVM channel by "registering" their token
//! accounts to the channel. This is done by delegating a token account to the
//! SolSVM on-chain program on Solana. This delegation is temporary, and
//! released immediately after channel settlement.
//!
//! Note: This opt-in solution is for demonstration purposes only.
//!
//! ```text
//! 
//! SolSVM "VM"
//!
//!    Bob          Alice        Bob          Alice          Will
//!     |             |           |             |             |
//!     | --o--o--o-> |           | --o--o--o-> |             |
//!     |             |           |             | --o--o--o-> | <--- SolSVM
//!     | <-o--o--o-- |           | <-o--o--o-- |             |    Transactions
//!     |             |           |             |             |
//!     | --o--o--o-> |           |     -----o--o--o----->    |
//!     |             |           |                           |
//!     | --o--o--o-> |           |     <----o--o--o------    |
//!
//!       \        /                  \         |         /
//!
//!         ------                           ------
//!        Alice: x                         Alice: x
//!        Bob:   x                         Bob:   x    <--- Solana Transaction
//!                                         Will:  x         with final ledgers
//!         ------                           ------
//!
//!           \\                               \\
//!            x                                x
//!
//!         Solana                           Solana     <--- Settled to Solana
//! ```
//!
//! The Solana SVM's `TransactionBatchProcessor` requires projects to provide a
//! "loader" plugin, which implements the `TransactionProcessingCallback`
//! interface.
//!
//! SolSVM defines a `SolSVMAccountLoader` that implements the
//! `TransactionProcessingCallback` interface, and provides it to the
//! `TransactionBatchProcessor` to process SolSVM transactions.

mod loader;
mod processor;
mod settler;
pub mod transaction;

use {
    crate::{
        loader::SolSVMAccountLoader, settler::SolSVMSettler, transaction::SolSVMTransaction,
    },
    processor::{create_transaction_batch_processor, get_transaction_check_results},
    solana_client::rpc_client::RpcClient,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_sdk::{
        feature_set::FeatureSet, fee::FeeStructure, hash::Hash, rent_collector::RentCollector,
        signature::Keypair,
    },
    solana_svm::transaction_processor::{
        TransactionProcessingConfig, TransactionProcessingEnvironment,
    },
    std::sync::Arc,
    transaction::create_svm_transactions,
};

/// A SolSVM channel instance.
///
/// Facilitates native SOL or SPL token transfers amongst various channel
/// participants, settling the final changes in balances to the base chain.
pub struct SolSVMChannel {
    /// I think you know why this is a bad idea...
    keys: Vec<Keypair>,
    rpc_client: RpcClient,
}

impl SolSVMChannel {
    pub fn new(keys: Vec<Keypair>, rpc_client: RpcClient) -> Self {
        Self { keys, rpc_client }
    }

    /// The SolSVM API. Processes a batch of SolSVM transactions.
    ///
    /// Obviously this is a very simple implementation, but one could imagine
    /// a more complex service that employs custom functionality, such as:
    ///
    /// * Increased throughput for individual P2P transfers.
    /// * Custom Solana transaction ordering (e.g. MEV).
    ///
    /// The general scaffold of the SolSVM API would remain the same.
    pub fn process_solsystem_transfers(&self, transactions: &[SolSVMTransaction]) {
        // SolSVM default configs.
        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let fee_structure = FeeStructure::default();
        let lamports_per_signature = fee_structure.lamports_per_signature;
        let rent_collector = RentCollector::default();

        // SolSVM loader/callback implementation.
        let account_loader = SolSVMAccountLoader::new(&self.rpc_client);

        // Solana SVM transaction batch processor.
        let processor =
            create_transaction_batch_processor(&account_loader, &feature_set, &compute_budget);

        // The SolSVM transaction processing runtime environment.
        let processing_environment = TransactionProcessingEnvironment {
            blockhash: Hash::default(),
            epoch_total_stake: None,
            epoch_vote_accounts: None,
            feature_set: Arc::new(feature_set),
            fee_structure: Some(&fee_structure),
            lamports_per_signature,
            rent_collector: Some(&rent_collector),
        };

        // The SolSVM transaction processing config for Solana SVM.
        let processing_config = TransactionProcessingConfig {
            compute_budget: Some(compute_budget),
            ..Default::default()
        };

        // 1. Convert to an SVM transaction batch.
        let svm_transactions = create_svm_transactions(transactions);

        // 2. Process transactions with the SVM API.
        let results = processor.load_and_execute_sanitized_transactions(
            &account_loader,
            &svm_transactions,
            get_transaction_check_results(svm_transactions.len(), lamports_per_signature),
            &processing_environment,
            &processing_config,
        );

        // 3. Convert results into a final ledger using a `SolSVMSettler`.
        let settler = SolSVMSettler::new(&self.rpc_client);

        // 4. Submit to the Solana base chain.
        settler.process_settle(transactions, results, &self.keys);
    }
}
