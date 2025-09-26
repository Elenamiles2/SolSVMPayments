mod setup;

use {
    solsvm::{transaction::SolSVMTransaction, SolSVMChannel},
    setup::{system_account, TestValidatorContext},
    solana_sdk::{signature::Keypair, signer::Signer},
};

#[test]
fn test_multi_hop_sol_transfers() {
    let alice = Keypair::new();
    let bob = Keypair::new();
    let will = Keypair::new();

    let alice_pubkey = alice.pubkey();
    let bob_pubkey = bob.pubkey();
    let will_pubkey = will.pubkey();

    // Give everyone 10 SOL (lamports)
    let accounts = vec![
        (alice_pubkey, system_account(10_000_000)),
        (bob_pubkey, system_account(10_000_000)),
        (will_pubkey, system_account(10_000_000)),
    ];

    let context = TestValidatorContext::start_with_accounts(accounts);
    let test_validator = &context.test_validator;
    let payer = context.payer.insecure_clone();
    let rpc_client = test_validator.get_rpc_client();

    let solsvm_channel = SolSVMChannel::new(vec![payer, alice, bob, will], rpc_client);

    solsvm_channel.process_solsvm_transfers(&[
        // Alice -> Bob 3 SOL
        SolSVMTransaction {
            from: alice_pubkey,
            to: bob_pubkey,
            amount: 3_000_000,
            mint: None,
        },
        // Bob -> Will 2 SOL
        SolSVMTransaction {
            from: bob_pubkey,
            to: will_pubkey,
            amount: 2_000_000,
            mint: None,
        },
        // Will -> Alice 1 SOL
        SolSVMTransaction {
            from: will_pubkey,
            to: alice_pubkey,
            amount: 1_000_000,
            mint: None,
        },
    ]);

    // Final balances:
    // Alice: 10 - 3 + 1 = 8
    // Bob:   10 + 3 - 2 = 11
    // Will:  10 + 2 - 1 = 11
    let rpc_client = test_validator.get_rpc_client();

    assert_eq!(rpc_client.get_balance(&alice_pubkey).unwrap(), 8_000_000);
    assert_eq!(rpc_client.get_balance(&bob_pubkey).unwrap(), 11_000_000);
    assert_eq!(rpc_client.get_balance(&will_pubkey).unwrap(), 11_000_000);
}
