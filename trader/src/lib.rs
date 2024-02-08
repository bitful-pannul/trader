use alloy_consensus::TxKind;
use alloy_sol_types::{SolCall, SolValue};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use kinode_process_lib::{
    await_message, call_init,
    eth::{
        call, get_balance, get_block_number, get_chain_id, get_gas_price, get_transaction_count,
        send_raw_transaction,
    },
    get_state, println, set_state, Address, Message,
};

use alloy_primitives::{Address as EthAddress, Bytes, U256};
use alloy_rpc_types::{CallInput, CallRequest};
use alloy_signer::{k256::ecdsa::SigningKey, LocalWallet, Signer, SignerSync, Transaction, Wallet};

mod helpers;
use crate::helpers::{
    calls::{get_erc20_info, get_token_price, send_swap_call_request},
    contracts::{IUniswapV2Factory, FACTORY, ROUTER, WETH},
    encryption::{decrypt_data, encrypt_data},
};

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

#[derive(Debug, Serialize, Deserialize)]
enum TradeRequest {
    Buy,
    Info,
    Send { amount: u64, to: String },
}

fn handle_message(our: &Address, wallet: &mut Wallet<SigningKey>) -> anyhow::Result<()> {
    let message = await_message()?;

    match message {
        Message::Response { .. } => {
            return Err(anyhow::anyhow!("unexpected Response: {:?}", message));
        }
        Message::Request {
            ref source,
            ref body,
            ..
        } => match serde_json::from_slice::<TradeRequest>(body)? {
            TradeRequest::Info => {
                let address = wallet.address();
                let eth_balance = get_balance(wallet.address(), None)?;
                let gas_price = get_gas_price()?;
                let block_number = get_block_number()?;

                println!("+------------------+--------------------------------+");
                println!("| Field            | Value                          |");
                println!("+------------------+--------------------------------+");
                println!("| Address          | {:<30} |", address);
                println!("| ETH Balance      | {:<30} |", eth_balance.to::<u64>());
                println!("| Gas Price        | {:<30} |", gas_price.to::<u64>());
                println!("| Block Number     | {:<30} |", block_number);
                println!("+------------------+--------------------------------+");
            }
            TradeRequest::Buy => {
                println!("entered buy mode, enter contract address:");
                let contract_address = await_message()?;
                let contract_address = String::from_utf8(contract_address.body().to_vec())?;
                let contract_address = EthAddress::from_str(&contract_address)?;

                let chain_id = get_chain_id()?.to::<u64>();
                let (decimals, symbol) = get_erc20_info(contract_address)?;
                println!("getting WETH pair for {:?}", symbol);

                let WETH_ADDRESS = WETH.get(&chain_id).ok_or_else(|| {
                    anyhow::anyhow!("WETH not found for chain_id: {:?}", chain_id)
                })?;

                let FACTORY_ADDRESS = FACTORY.get(&chain_id).ok_or_else(|| {
                    anyhow::anyhow!("WETH not found for chain_id: {:?}", chain_id)
                })?;

                let ROUTER_ADDRESS = ROUTER.get(&chain_id).ok_or_else(|| {
                    anyhow::anyhow!("WETH not found for chain_id: {:?}", chain_id)
                })?;

                let func_call = IUniswapV2Factory::getPairCall {
                    tokenA: *WETH_ADDRESS,
                    tokenB: contract_address,
                }
                .abi_encode();
                // this should be a better builder...
                let req = CallRequest {
                    from: None,
                    to: Some(FACTORY_ADDRESS.clone()),
                    input: CallInput {
                        input: Some(func_call.into()),
                        data: None,
                    },
                    ..Default::default()
                };
                let res = call(req, None)?;

                let pair_address = EthAddress::abi_decode(&res, false)?;
                println!("got pair address: {:?}", pair_address);

                let (p0, p1) =
                    get_token_price(pair_address, "WETH", &symbol, 18, decimals.to::<u8>())?;
                println!("{:.4} {} per {}", p0, symbol, "WETH");
                println!("{:.4} {} per {}", p1, "WETH", symbol);

                println!("input how much you want to buy:");
                let amount_in = await_message()?;
                let amount_in = String::from_utf8(amount_in.body().to_vec())?.parse::<u64>()?;

                // manual 5% slippage rn. or 100% is there mev on sepolia?
                let amount_in_scaled = U256::from(amount_in);

                // let min_amount_out = p0 * amount_in_scaled;
                // let min_amount_out = U256::from(min_amount_out as u64);

                let path = vec![*WETH_ADDRESS, contract_address];

                let mut tx = send_swap_call_request(
                    wallet.address(),
                    chain_id,
                    *ROUTER_ADDRESS,
                    amount_in,
                    U256::from(0),
                    path,
                )?;

                let sig = wallet.sign_transaction_sync(&mut tx)?;
                let signed_tx = tx.into_signed(sig);

                let mut buf = vec![];
                signed_tx.encode_signed(&mut buf);

                let tx_hash = send_raw_transaction(buf.into())?;
                println!("sent! with tx_hash {:?}", tx_hash);
            }
            TradeRequest::Send { amount, to } => {
                let to = EthAddress::from_str(&to)?;
                let chain_id = get_chain_id()?;
                let gas_price = get_gas_price()?;
                let nonce = get_transaction_count(wallet.address(), None)?;

                let mut tx = alloy_consensus::TxLegacy {
                    nonce: nonce.to::<u64>(),
                    gas_price: gas_price.to::<u128>(),
                    gas_limit: 21000,
                    to: TxKind::Call(to), // Use `TxKind::Call` with the recipient's address
                    value: U256::from(amount),
                    input: Bytes::default(),
                    chain_id: Some(chain_id.to::<u64>()),
                };

                let sig = wallet.sign_transaction_sync(&mut tx)?;
                let signed_tx = tx.into_signed(sig);

                // is this necessary?
                let mut buf = vec![];
                signed_tx.encode_signed(&mut buf);

                let tx_hash = send_raw_transaction(buf.into())?;
                println!("sent! with tx_hash {:?}", tx_hash);
            }
        },
    }
    Ok(())
}

call_init!(init);
fn init(our: Address) {
    println!("trader: begin");

    // this block is essentially a messy CLI initialization app,
    // todo fix it up.
    // also todo, save pk in file, store bookmarks etc in state.
    let mut wallet = loop {
        let temp_wallet: Option<Wallet<SigningKey>>;

        if let Some(encrypted_state) = get_state() {
            println!("Enter password to unlock wallet:");
            let password_msg = await_message().unwrap();
            let password_str =
                String::from_utf8(password_msg.body().to_vec()).unwrap_or_else(|_| "".to_string());

            match decrypt_data(&encrypted_state, &password_str) {
                Ok(decrypted_state) => match String::from_utf8(decrypted_state)
                    .ok()
                    .and_then(|wd| wd.parse::<LocalWallet>().ok())
                {
                    Some(parsed_wallet) => {
                        println!(
                            "Trader: Loaded wallet with address: {:?}",
                            parsed_wallet.address()
                        );
                        temp_wallet = Some(parsed_wallet);
                        break temp_wallet; // Exit loop on success
                    }
                    None => println!("Failed to parse wallet, try again."),
                },
                Err(_) => println!("Decryption failed, try again."),
            }
        } else {
            println!("No wallet loaded, input a key:");
            let wallet_msg = await_message().unwrap();
            let wallet_data_str = String::from_utf8(wallet_msg.body().to_vec()).unwrap();

            println!("Input a password to save it:");
            let password_msg = await_message().unwrap();
            let password_str = String::from_utf8(password_msg.body().to_vec()).unwrap();

            let encrypted_wallet_data = encrypt_data(wallet_data_str.as_bytes(), &password_str);
            set_state(&encrypted_wallet_data);

            if let Ok(parsed_wallet) = wallet_data_str.parse::<LocalWallet>() {
                println!(
                    "Trader: Loaded wallet with address: {:?}",
                    parsed_wallet.address()
                );
                temp_wallet = Some(parsed_wallet);
                break temp_wallet; // Exit loop on success
            } else {
                println!("Failed to parse wallet key, try again.");
            }
        }
    }
    .expect("Failed to initialize wallet");

    loop {
        match handle_message(&our, &mut wallet) {
            Ok(()) => {}
            Err(e) => {
                println!("trader: error: {:?}", e);
            }
        };
    }
}
