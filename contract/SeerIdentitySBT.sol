// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract SeerIdentitySBT {
    address public owner;
    address public backendSigner;
    uint256 public nextTokenId = 1;

    mapping(uint256 => address) public ownerOf;
    mapping(address => uint256) public tokenOfOwner;
    mapping(uint256 => string) public tokenURI;

    event IdentityMinted(address indexed user, uint256 indexed tokenId, string uri);
    event TokenURIUpdated(uint256 indexed tokenId, string uri);

    modifier onlyOwner() {
        require(msg.sender == owner, "NOT_OWNER");
        _;
    }

    modifier onlyBackendSigner() {
        require(msg.sender == backendSigner, "NOT_BACKEND_SIGNER");
        _;
    }

    constructor(address signer) {
        require(signer != address(0), "ZERO_SIGNER");
        owner = msg.sender;
        backendSigner = signer;
    }

    function setBackendSigner(address signer) external onlyOwner {
        require(signer != address(0), "ZERO_SIGNER");
        backendSigner = signer;
    }

    function mintIdentity(address user, string calldata uri) external onlyBackendSigner returns (uint256) {
        require(user != address(0), "ZERO_USER");
        require(tokenOfOwner[user] == 0, "ALREADY_MINTED");

        uint256 tokenId = nextTokenId++;
        ownerOf[tokenId] = user;
        tokenOfOwner[user] = tokenId;
        tokenURI[tokenId] = uri;

        emit IdentityMinted(user, tokenId, uri);
        return tokenId;
    }

    function updateTokenURI(uint256 tokenId, string calldata uri) external onlyBackendSigner {
        require(ownerOf[tokenId] != address(0), "UNKNOWN_TOKEN");
        tokenURI[tokenId] = uri;
        emit TokenURIUpdated(tokenId, uri);
    }

    function transferFrom(address, address, uint256) external pure {
        revert("SOULBOUND");
    }
}
