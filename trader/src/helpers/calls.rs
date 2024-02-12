use alloy_consensus::{TxKind, TxLegacy};
use alloy_sol_types::{SolCall, SolValue};
use kinode_process_lib::eth::{
    call, get_gas_price, get_transaction_count, Address, TransactionInput, TransactionRequest,
    U256, U8,
};

use crate::helpers::contracts::{IUniswapV2Pair, IUniswapV2Router01, IERC20};

pub fn get_erc20_info(address: Address) -> anyhow::Result<(U8, String)> {
    let decimals_call = IERC20::decimalsCall {}.abi_encode();
    let decimals_req = TransactionRequest {
        to: Some(address),
        input: TransactionInput::new(decimals_call.into()),
        ..Default::default()
    };
    let decimals_res = call(decimals_req, None)?;

    let symbol_call = IERC20::symbolCall {}.abi_encode();

    let symbol_req = TransactionRequest {
        to: Some(address),
        input: TransactionInput::new(symbol_call.into()),
        ..Default::default()
    };
    let symbol_res = call(symbol_req, None)?;

    let symbol = String::abi_decode(&symbol_res, false)?;
    // apparently U8 decoding not implemented..
    let decimals = U256::abi_decode(&decimals_res, false)?;
    let decimals = decimals.to::<U8>();

    Ok((decimals, symbol))
}

pub fn get_token_price(
    pair_address: Address,
    token0_symbol: &str,
    token1_symbol: &str,
    token0_decimals: u8,
    token1_decimals: u8,
) -> anyhow::Result<(f64, f64)> {
    // Encode the call to getReserves on the pair contract
    let get_reserves_call = IUniswapV2Pair::getReservesCall {}.abi_encode();
    let reserves_req = TransactionRequest {
        to: Some(pair_address),
        input: TransactionInput::new(get_reserves_call.into()),
        ..Default::default()
    };
    let reserves_res = call(reserves_req, None)?;

    // Decode the reserves
    let (reserve0, reserve1, _timestamp) = <(U256, U256, U256)>::abi_decode(&reserves_res, false)?;

    // Convert U256 reserves to f64, adjusting for token decimals
    let adjusted_reserve0 = reserve0.to::<u128>() as f64 / 10f64.powi(token0_decimals.into());
    let adjusted_reserve1 = reserve1.to::<u128>() as f64 / 10f64.powi(token1_decimals.into());

    // Calculate price of token0 in terms of token1 and vice versa
    let price0_in_terms_of_1 = if adjusted_reserve1 > 0.0 {
        adjusted_reserve0 / adjusted_reserve1
    } else {
        return Err(anyhow::anyhow!("Reserve1 is zero, cannot calculate price."));
    };

    let price1_in_terms_of_0 = if adjusted_reserve0 > 0.0 {
        adjusted_reserve1 / adjusted_reserve0
    } else {
        return Err(anyhow::anyhow!("Reserve0 is zero, cannot calculate price."));
    };

    Ok((price0_in_terms_of_1, price1_in_terms_of_0))
}

pub fn send_swap_call_request(
    from: Address,           // Address of the sender
    chain_id: u64,           // Chain ID
    router_address: Address, // Address of the Uniswap router
    amount_in: u64,          // Amount of ETH to swap
    min_amount_out: U256,    // Minimum amount of the other token you're willing to accept
    path: Vec<Address>,      // Path of the swap (ETH -> Other Token)
) -> anyhow::Result<TxLegacy> {
    // Encode the call to swapExactETHForTokens
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 60 * 20;

    let swap_call = IUniswapV2Router01::swapExactETHForTokensCall {
        amountOutMin: min_amount_out,
        path: path,
        to: from,
        deadline: U256::from(deadline),
    }
    .abi_encode();

    let gas_price = get_gas_price()?;
    let nonce = get_transaction_count(from, None)?;

    let tx = TxLegacy {
        nonce: nonce.to::<u64>(),
        gas_price: gas_price.to::<u128>() * 8,
        gas_limit: 220000,
        to: TxKind::Call(router_address),
        value: U256::from(amount_in),
        input: swap_call.into(),
        chain_id: Some(chain_id),
    };

    Ok(tx)
}
