mod setup;

use {
    solsvm::{transaction::SolSVMTransaction, SolSVMChannel},
    setup::{get_token_account_balance, mint_account, system_account, token_account, TestValidatorContext},
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
    spl_associated_token_account::get_associated_token_address,
};

#[test]
fn test_mixed_sol_and_spl() {
    let mint = Pubkey::new_unique();

    let alice = Keypair::new();
    let bob = Keypair::new();

    let alice_pubkey = alice.pubkey();
    let alice_token_pubkey = get_associated_token_address(&alice_pubkey, &mint);

    let bob_pubkey = bob.pubkey();
    let bob_token_pubkey = get_associated_token_address(&bob_pubkey, &mint);

    // Accounts: both SOL balances + SPL token accounts
    let accounts = vec![
        (mint, mint_account()),
        (alice_pubkey, system_account(10_000_000)),
        (alice_token_pubkey, token_account(&alice_pubkey, &mint, 5)),
        (bob_pubkey, system_account(10_000_000)),
        (bob_token_pubkey, token_account(&bob_pubkey, &mint, 5)),
    ];

    let context = TestValidatorContext::start_with_accounts(accounts);
    let test_validator = &context.test_validator;
    let payer = context.payer.insecure_clone();

    let rpc_client = test_validator.get_rpc_client();
    let solsvm_channel = SolSVMChannel::new(vec![payer, alice, bob], rpc_client);

    solsvm_channel.process_solsvm_transfers(&[
        // Alice -> Bob 2 SOL
        SolSVMTransaction {
            from: alice_pubkey,
            to: bob_pubkey,
            amount: 2_000_000,
            mint: None,
        },
        // Bob -> Alice 3 SPL tokens
        SolSVMTransaction {
            from: bob_pubkey,
            to: alice_pubkey,
            amount: 3,
            mint: Some(mint),
        },
    ]);

    // Expected final balances:
    // Alice: SOL 8_000_000 (down 2_000_000), SPL 8 (5 + 3)
    // Bob:   SOL 12_000_000 (up 2_000_000), SPL 2 (5 - 3)

    let rpc_client = test_validator.get_rpc_client();

    assert_eq!(rpc_client.get_balance(&alice_pubkey).unwrap(), 8_000_000);
    assert_eq!(rpc_client.get_balance(&bob_pubkey).unwrap(), 12_000_000);

    assert_eq!(
        get_token_account_balance(rpc_client.get_account(&alice_token_pubkey).unwrap()),
        8
    );
    assert_eq!(
        get_token_account_balance(rpc_client.get_account(&bob_token_pubkey).unwrap()),
        2
    );
}
