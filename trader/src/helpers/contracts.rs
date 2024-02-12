use alloy_sol_types::sol;
use kinode_process_lib::eth::Address;
use lazy_static::lazy_static;
use std::collections::HashMap;
// here we define the types from the ABIs we need.
// we could compile the contracts, and import their entire JSONs,
// but here we go the lightweight route and only define the functions that we need!

// contract addresses for Sepolia and Optimism (for now)
lazy_static! {
    pub static ref WETH: HashMap<u64, Address> = {
        let mut m = HashMap::new();
        m.insert(10, "0x4200000000000000000000000000000000000006".parse::<Address>().unwrap()); // Optimism
        m.insert(11155111, "0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9".parse::<Address>().unwrap()); // Sepolia
        m
    };

    pub static ref FACTORY: HashMap<u64, Address> = {
        let mut m = HashMap::new();
        m.insert(10, "0x7E0987E5b3a30e3f2828572Bb659A548460a3003".parse::<Address>().unwrap());
        m.insert(11155111,"0x7E0987E5b3a30e3f2828572Bb659A548460a3003".parse::<Address>().unwrap());
        m
    };

    pub static ref ROUTER: HashMap<u64, Address> = {
        let mut m = HashMap::new();
        m.insert(10, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>().unwrap());
        m.insert(11155111, "0xC532a74256D3Db42D0Bf7a0400fEFDbad7694008".parse::<Address>().unwrap());
        m
    };
}

sol! {
    /// Interface of the ERC20 standard as defined in [the EIP].
    ///
    /// [the EIP]: https://eips.ethereum.org/EIPS/eip-20
    #[derive(Debug)]
    interface IERC20 {
        event Approval(address indexed owner, address indexed spender, uint value);
        event Transfer(address indexed from, address indexed to, uint value);

        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function balanceOf(address owner) external view returns (uint);

        function totalSupply() external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}

sol! {
    #[derive(Debug)]
    interface IUniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
    }
}

sol! {
    #[derive(Debug)]
    interface IUniswapV2Pair {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }
}

sol! {
    #[derive(Debug)]
    interface IUniswapV2Router01 {
        function swapExactTokensForTokens(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);
        function swapTokensForExactTokens(
            uint amountOut,
            uint amountInMax,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
            external
            payable
            returns (uint[] memory amounts);
        function swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline)
            external
            returns (uint[] memory amounts);
        function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline)
            external
            returns (uint[] memory amounts);
        function swapETHForExactTokens(uint amountOut, address[] calldata path, address to, uint deadline)
            external
            payable
            returns (uint[] memory amounts);

        function quote(uint amountA, uint reserveA, uint reserveB) external pure returns (uint amountB);
        function getAmountOut(uint amountIn, uint reserveIn, uint reserveOut) external pure returns (uint amountOut);
        function getAmountIn(uint amountOut, uint reserveIn, uint reserveOut) external pure returns (uint amountIn);
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts);
        function getAmountsIn(uint amountOut, address[] calldata path) external view returns (uint[] memory amounts);
    }
}
