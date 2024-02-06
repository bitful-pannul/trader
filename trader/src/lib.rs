use alloy_consensus::TxKind;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use kinode_process_lib::{
    await_message, call_init,
    eth::{
        get_balance, get_block_number, get_chain_id, get_gas_price, get_transaction_count,
        send_raw_transaction,
    },
    get_state, println, set_state, Address, Message, ProcessId, Request, Response,
};

use alloy_primitives::{Address as EthAddress, Bytes, U256};

use alloy_signer::{
    k256::{ecdsa::SigningKey, Secp256k1},
    LocalWallet, Signer, SignerSync, Transaction, Wallet,
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
    Info,
    SetAccount,
    Password,
    Balance,
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
            TradeRequest::Balance => {
                let eth_balance = get_balance(wallet.address(), None)?;
                println!("account eth balance: {}", eth_balance.to::<u64>());
            }
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

            TradeRequest::SetAccount => {}
            TradeRequest::Password => {}
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

    let mut wallet = {
        let mut temp_wallet: Option<Wallet<SigningKey>> = None;

        // check if there's existing state to load the wallet from
        // todo encrypt, store in file instead?
        if let Some(state) = get_state() {
            let pk_string = String::from_utf8(state).unwrap();
            let wallet = pk_string.parse::<LocalWallet>().unwrap();
            temp_wallet = Some(wallet);
            println!(
                "loaded wallet: {:?}",
                temp_wallet.as_ref().unwrap().address()
            );
        } else {
            // No existing state, prompt for wallet input
            println!("no wallet loaded, input a key");
            loop {
                let msg = await_message().unwrap();
                if let Ok(s) = String::from_utf8(msg.body().to_vec()) {
                    if let Ok(parsed_wallet) = s.parse::<LocalWallet>() {
                        println!(
                            "trader: loaded wallet with address: {:?}",
                            parsed_wallet.address()
                        );
                        temp_wallet = Some(parsed_wallet);
                        set_state(s.as_bytes());
                        break;
                    } else {
                        println!("trader: failed key parse..");
                    }
                } else {
                    println!("trader: failed key msg parse..");
                }
            }
        }

        // Move the wallet out of the Option, assuming it's been initialized at this point
        temp_wallet.unwrap()
    };

    loop {
        match handle_message(&our, &mut wallet) {
            Ok(()) => {}
            Err(e) => {
                println!("trader: error: {:?}", e);
            }
        };
    }
}
