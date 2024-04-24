// https://github.com/Genesis3800/CREATE2Factory/blob/b202029eadc0299e6e5923dd90db4200c2f7955a/src/Create2.sol

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Create2 {

    error Create2InsufficientBalance(uint256 received, uint256 minimumNeeded);

    error Create2EmptyBytecode();

    error Create2FailedDeployment();

    function deploy(uint256 amount, bytes32 salt, bytes memory bytecode) external payable returns (address addr) {
        
        if (msg.value < amount) {
            revert Create2InsufficientBalance(msg.value, amount);
        }

        if (bytecode.length == 0) {
            revert Create2EmptyBytecode();
        }

        assembly {
            addr := create2(amount, add(bytecode, 0x20), mload(bytecode), salt)
        }

        if (addr == address(0)) {
            revert Create2FailedDeployment();
        }
    }

    function computeAddress(bytes32 salt, bytes32 bytecodeHash) external view returns (address addr) {

        address contractAddress = address(this);
        
        assembly {
            let ptr := mload(0x40)

            mstore(add(ptr, 0x40), bytecodeHash)
            mstore(add(ptr, 0x20), salt)
            mstore(ptr, contractAddress)
            let start := add(ptr, 0x0b)
            mstore8(start, 0xff)
            addr := keccak256(start, 85)
        }
    }

}
