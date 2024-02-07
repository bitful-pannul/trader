use alloy_primitives::Address;
use alloy_sol_types::{SolCall, SolValue};
use kinode_process_lib::eth::call;

use alloy_primitives::{U256, U8};
use alloy_rpc_types::{CallInput, CallRequest};

use crate::helpers::contracts::{IUniswapV2Pair, IERC20};

pub fn get_erc20_info(address: Address) -> anyhow::Result<(U8, String)> {
    let decimals_call = IERC20::decimalsCall {}.abi_encode();
    let decimals_req = CallRequest {
        from: None,
        to: Some(address),
        input: CallInput {
            input: Some(decimals_call.into()),
            data: None,
        },
        ..Default::default()
    };
    let decimals_res = call(decimals_req, None)?;

    let symbol_call = IERC20::symbolCall {}.abi_encode();

    let symbol_req = CallRequest {
        from: None,
        to: Some(address),
        input: CallInput {
            input: Some(symbol_call.into()),
            data: None,
        },
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
    let reserves_req = CallRequest {
        from: None,
        to: Some(pair_address),
        input: CallInput {
            input: Some(get_reserves_call.into()),
            data: None,
        },
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

    // Format the price information
    let formatted_price = format!(
        "{:.4} {} per {} | {:.4} {} per {}",
        price0_in_terms_of_1,
        token1_symbol,
        token0_symbol,
        price1_in_terms_of_0,
        token0_symbol,
        token1_symbol
    );

    Ok((price0_in_terms_of_1, price1_in_terms_of_0))
}
