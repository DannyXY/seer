// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// Minimal mintable ERC-20 used to validate Seer's execution pipeline on
/// Mantle Sepolia, where canonical tokens are not deployed. 6 decimals to
/// mirror USDC semantics.
contract SeerTestToken {
    string public name = "Seer Test USDC";
    string public symbol = "USDC";
    uint8 public constant decimals = 6;
    uint256 public totalSupply;

    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function mint(address to, uint256 amount) external {
        totalSupply += amount;
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        return _transfer(msg.sender, to, amount);
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        require(allowed >= amount, "INSUFFICIENT_ALLOWANCE");
        if (allowed != type(uint256).max) {
            allowance[from][msg.sender] = allowed - amount;
        }
        return _transfer(from, to, amount);
    }

    function _transfer(address from, address to, uint256 amount) internal returns (bool) {
        require(balanceOf[from] >= amount, "INSUFFICIENT_BALANCE");
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
        return true;
    }
}

/// Minimal strategy implementing the generic `deposit(address,uint256)`
/// signature Seer's transaction builder emits. Pulls approved tokens from the
/// caller and records the deposit, proving the approve -> deposit flow
/// end-to-end on-chain.
contract SeerTestStrategy {
    mapping(address => mapping(address => uint256)) public deposits;

    event Deposited(address indexed user, address indexed token, uint256 amount);

    function deposit(address token, uint256 amount) external {
        require(amount > 0, "ZERO_AMOUNT");
        (bool ok, bytes memory data) = token.call(
            abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), amount)
        );
        require(ok && (data.length == 0 || abi.decode(data, (bool))), "TRANSFER_FROM_FAILED");
        deposits[msg.sender][token] += amount;
        emit Deposited(msg.sender, token, amount);
    }
}
