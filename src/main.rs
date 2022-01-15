use {
    anyhow::Result,
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        program_pack::Pack,
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account::{create_associated_token_account, get_associated_token_address},
    spl_token::state::Account as TokenAccount,
    spl_token_lending::instruction::{
        deposit_reserve_liquidity, redeem_reserve_collateral, refresh_reserve,
    },
    std::{str::FromStr, thread::sleep, time::Duration},
};

// Example account: https://explorer.solana.com/address/3FAJ6YenHtCfQZsDtcn6NvZATnmMGafs39x8BbNSWRwS
// Deposit: https://explorer.solana.com/tx/e5movuPEe4XAPx2qoYUHbjaQa95Z6FTF7Gf1jL8Nb3YsWYjFy8j7zpyxbx7yZTfxE1dHDVpCqGb7sdTWcwsdaRo
// Withdraw: https://explorer.solana.com/tx/XV2NKAYFB4G8fKJNV7vgDWyFFXvqabZrYCJ5UxdPmUJeowXMhN6Uvg4QhkEP9LtxEWNsM2dA2rvYtGacGQySeft

fn main() -> Result<()> {
    let owner =
        "YAQgu3cRyXuYnXW2V4otrV7ePXGq2jwWJ5gC8y5c3uE8Kssv6f5LwJ8JUyrdJhNeNhAwv8XmUtPWGabEx22BYrE";
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
    let tuusdc_mint = Pubkey::from_str("Amig8TisuLpzun8XyGfC5HJHHGUQEscjLgoTWsCCKihg")?;
    let program_id = Pubkey::from_str("4bcFeLv4nydFrsZqV5CgwCVrPhkQKsXtzfy2KyMz7ozM")?;
    let reserve = Pubkey::from_str("FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt")?;
    let reserve_liquidity_oracle =
        Pubkey::from_str("ExzpbWgczTgd8J58BrnESndmzBkRVfc6PhFjSGiQXgAB")?;
    let reserve_liquidity_supply =
        Pubkey::from_str("64QJd6MYXUjCBvCaZKaqxiKmaMkPUdNonE1KuY1YoGGb")?;
    let lending_market = Pubkey::from_str("D1cqtVThyebK9KXKGXrCEuiqaNf5L4UfM1vHgCqiJxym")?;

    let owner = Keypair::from_base58_string(owner);
    let owner_pubkey = owner.pubkey();
    let owner_usdc = get_associated_token_address(&owner_pubkey, &usdc_mint);
    let owner_tuusdc = get_associated_token_address(&owner_pubkey, &tuusdc_mint);

    println!("SOL: {:?}", owner.pubkey());
    println!("USDC: {:?}", owner_usdc);
    println!("tuUSDC: {:?}", owner_tuusdc);

    let client = RpcClient::new_with_commitment(
        "https://solana-api.projectserum.com".to_owned(),
        CommitmentConfig::processed(),
    );
    println!("Connected to validator, ver: {:?}", client.get_version()?);

    // Check SOL balance
    let sol_account = client.get_account(&owner_pubkey)?;
    println!(
        "SOL balance: {:#.9}",
        sol_account.lamports as f64 / 10f64.powi(9)
    );

    // Check USDC balance
    let usdc_account = client.get_account(&owner_usdc)?;
    let usdc_account = TokenAccount::unpack_from_slice(&usdc_account.data)?;
    println!(
        "USDC balance: {:#.6}",
        usdc_account.amount as f64 / 10f64.powi(6)
    );

    // Check tuUSDC balance
    let tuusdc_account = client.get_account(&owner_tuusdc)?;
    let tuusdc_account = TokenAccount::unpack_from_slice(&tuusdc_account.data)?;
    println!(
        "tuUSDC balance: {:#.6}",
        tuusdc_account.amount as f64 / 10f64.powi(6)
    );

    // Deposit
    if usdc_account.amount > 0 {
        let mut instructions = vec![];
        match client.get_account(&owner_tuusdc) {
            Ok(_) => {}
            Err(error) if error.to_string().contains("AccountNotFound") => {
                instructions.push(create_associated_token_account(
                    &owner_pubkey,
                    &owner_pubkey,
                    &tuusdc_mint,
                ));
            }
            Err(error) => panic!("{}", error),
        }
        instructions.extend_from_slice(&[
            refresh_reserve(program_id, reserve, reserve_liquidity_oracle),
            deposit_reserve_liquidity(
                program_id,
                usdc_account.amount,
                owner_usdc,
                owner_tuusdc,
                reserve,
                reserve_liquidity_supply,
                tuusdc_mint,
                lending_market,
                owner_pubkey,
            ),
        ]);
        let mut tx = Transaction::new_with_payer(&instructions, Some(&owner_pubkey));
        let blockhash = client.get_recent_blockhash()?.0;
        tx.sign(&[&owner], blockhash);
        let signature = client.send_transaction(&tx)?;
        println!("deposit tx signature: {}", signature);
        while !client.confirm_transaction(&signature)? {
            println!("wait transaction confirmation...");
            sleep(Duration::from_secs(3));
        }
    } else {
        println!("noting to deposit... skip");
    }

    // Check tuUSDC balance
    let tuusdc_account = client.get_account(&owner_tuusdc)?;
    let tuusdc_account = TokenAccount::unpack_from_slice(&tuusdc_account.data)?;
    println!(
        "tuUSDC balance: {:#.6}",
        tuusdc_account.amount as f64 / 10f64.powi(6)
    );

    // Withdraw
    if tuusdc_account.amount > 0 {
        let mut tx = Transaction::new_with_payer(
            &[
                refresh_reserve(program_id, reserve, reserve_liquidity_oracle),
                redeem_reserve_collateral(
                    program_id,
                    tuusdc_account.amount,
                    owner_tuusdc,
                    owner_usdc,
                    reserve,
                    tuusdc_mint,
                    reserve_liquidity_supply,
                    lending_market,
                    owner_pubkey,
                ),
            ],
            Some(&owner_pubkey),
        );
        let blockhash = client.get_recent_blockhash()?.0;
        tx.sign(&[&owner], blockhash);
        let signature = client.send_transaction(&tx)?;
        println!("withdraw tx signature: {}", signature);
        while !client.confirm_transaction(&signature)? {
            println!("wait transaction confirmation...");
            sleep(Duration::from_secs(3));
        }
    } else {
        println!("noting to withdraw... skip");
    }

    // Check SOL balance
    let sol_account = client.get_account(&owner_pubkey)?;
    println!(
        "SOL balance: {:#.9}",
        sol_account.lamports as f64 / 10f64.powi(9)
    );

    // Check USDC balance
    let usdc_account = client.get_account(&owner_usdc)?;
    let usdc_account = TokenAccount::unpack_from_slice(&usdc_account.data)?;
    println!(
        "USDC balance: {:#.6}",
        usdc_account.amount as f64 / 10f64.powi(6)
    );

    // Check tuUSDC balance
    let tuusdc_account = client.get_account(&owner_tuusdc)?;
    let tuusdc_account = TokenAccount::unpack_from_slice(&tuusdc_account.data)?;
    println!(
        "tuUSDC balance: {:#.6}",
        tuusdc_account.amount as f64 / 10f64.powi(6)
    );

    // // Optional, move funds out
    // let (blockhash, fee_calculator) = client.get_recent_blockhash()?;
    // let receiver = Pubkey::from_str("FTqaWjTNTM35eWwxE64zmxzqZXFcLH5t7bvRqJCcTXWU")?;
    // let receiver_usdc = get_associated_token_address(&receiver, &usdc_mint);
    // let mut instructions = vec![];
    // if usdc_account.amount > 0 {
    //     instructions.push(spl_token::instruction::transfer(
    //         &spl_token::ID,
    //         &owner_usdc,
    //         &receiver_usdc,
    //         &owner_pubkey,
    //         &[],
    //         usdc_account.amount,
    //     )?);
    // }
    // instructions.extend_from_slice(&[
    //     spl_token::instruction::close_account(
    //         &spl_token::ID,
    //         &owner_usdc,
    //         &receiver,
    //         &owner_pubkey,
    //         &[],
    //     )?,
    //     spl_token::instruction::close_account(
    //         &spl_token::ID,
    //         &owner_tuusdc,
    //         &receiver,
    //         &owner_pubkey,
    //         &[],
    //     )?,
    //     solana_sdk::system_instruction::transfer(
    //         &owner_pubkey,
    //         &receiver,
    //         sol_account.lamports - fee_calculator.lamports_per_signature,
    //     ),
    // ]);
    // let mut tx = Transaction::new_with_payer(&instructions, Some(&owner_pubkey));
    // tx.sign(&[&owner], blockhash);
    // let signature = client.send_transaction(&tx)?;
    // println!("move funds tx signature: {}", signature);

    Ok(())
}
