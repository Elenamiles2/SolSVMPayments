//! SolSVM's custom transaction format, tailored specifically for SOL or SPL
//! token transfers.
//!
//! Mostly for demonstration purposes, to show how projects may use completely
//! different transactions in their protocol, then convert the resulting state
//! transitions into the necessary transactions for the base chain - in this
//! case Solana.

use {
    solana_sdk::{
        instruction::Instruction as Solanalnstruction,
        pubkey::Pubkey,
        system_instruction,
        transaction::{
            SanitizedTransaction as SolanaSanitizedTransaction, Transaction as SolanaTransaction,
        },
    },
    spl_associated_token_account::get_associated_token_address,
    std::collections::HashSet,
};

/// A simple SolSVM transaction. Transfers SPL tokens or SOL from one account
/// to another.
///
/// A `None` value for `mint` represents native SOL.
pub struct SolSVMTransaction {
    pub mint: Option<Pubkey>,
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

impl From<&SolSVMTransaction> for SolanaInstruction {
    fn from(value: &SolSVMTransaction) -> Self {
        let SolSVMTransaction {
            mint,
            from,
            to,
            amount,
        } = value;
        if let Some(mint) = mint {
            let source_pubkey = get_associated_token_address(from, mint);
            let destination_pubkey = get_associated_token_address(to, mint);
            return spl_token::instruction::transfer(
                &spl_token::id(),
                &source_pubkey,
                &destination_pubkey,
                from,
                &[],
                *amount,
            )
            .unwrap();
        }
        system_instruction::transfer(from, to, *amount)
    }
}

impl From<&SolSVMTransaction> for SolanaTransaction {
    fn from(value: &SolSVMTransaction) -> Self {
        SolanaTransaction::new_with_payer(&[SolanaInstruction::from(value)], Some(&value.from))
    }
}

impl From<&SolSVMTransaction> for SolanaSanitizedTransaction {
    fn from(value: &SolSVMTransaction) -> Self {
        SolanaSanitizedTransaction::try_from_legacy_transaction(
            SolanaTransaction::from(value),
            &HashSet::new(),
        )
        .unwrap()
    }
}

/// Create a batch of Solana transactions, for the Solana SVM's transaction
/// processor, from a batch of SolSVM instructions.
pub fn create_svm_transactions(
    solsvm_transactions: &[SolSVMTransaction],
) -> Vec<SolanaSanitizedTransaction> {
    solsvm_transactions
        .iter()
        .map(SolanaSanitizedTransaction::from)
        .collect()
}
