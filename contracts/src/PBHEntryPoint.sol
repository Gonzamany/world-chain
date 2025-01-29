// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

/// @title PBH Entry Point
/// @author Worldcoin
/// @notice An implementation of an on chain PBH Entry Point.
/// @custom:security-contact security@toolsforhumanity.com
contract PBHEntryPoint is ERC1967Proxy {
    ///////////////////////////////////////////////////////////////////////////////
    ///                    !!!! DO NOT ADD MEMBERS HERE !!!!                    ///
    ///////////////////////////////////////////////////////////////////////////////

    ///////////////////////////////////////////////////////////////////////////////
    ///                             CONSTRUCTION                                ///
    ///////////////////////////////////////////////////////////////////////////////

    /// @notice Constructs a new instance of the PBH Entry Point.
    /// @dev This constructor is only called once, and can be called with the encoded call necessary
    ///      to initialize the logic contract.
    ///
    /// @param _logic The initial implementation (delegate) of the contract that this acts as a proxy
    ///        for.
    /// @param _data If this is non-empty, it is used as the data for a `delegatecall` to `_logic`.
    ///        This is usually an encoded function call, and allows for initialising the storage of
    ///        the proxy in a way similar to a traditional solidity constructor.
    constructor(address _logic, bytes memory _data) payable ERC1967Proxy(_logic, _data) {
        // !!!! DO NOT PUT PROGRAM LOGIC HERE !!!!
        // It should go in the `initialize` function of the delegate instead.
    }
}
