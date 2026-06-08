/**
 * Send a backend-prepared calldata object using the Privy wallet provider.
 * Auto-switches to the correct chain before sending.
 * Supports both injected wallets (MetaMask) and Privy embedded wallets (Google OAuth).
 */
export async function sendOnChainTx(calldata) {
  const { to, data, chain_id } = calldata;
  if (!window.privyEthersProvider) throw new Error("No wallet provider available.");
  let provider = window.privyEthersProvider;

  if (chain_id) {
    const network = await provider.getNetwork();
    if (network.chainId !== chain_id) {
      await switchToChain(chain_id);
      // Always get a fresh provider after a chain switch
      if (window.privyWallet?.getEthersProvider) {
        provider = await window.privyWallet.getEthersProvider();
        window.privyEthersProvider = provider;
      }
    }
  }

  const signer = await provider.getSigner();
  const tx = await signer.sendTransaction({ to, data });
  return tx.hash;
}

/**
 * Switch to the given chainId (number).
 * Privy embedded wallets use wallet.switchChain(); injected wallets use
 * wallet_switchEthereumChain / wallet_addEthereumChain.
 */
async function switchToChain(chainId) {
  const chainHex = "0x" + chainId.toString(16);
  const wallet = window.privyWallet;

  // Privy embedded wallet (Google / email / passkey login) — use Privy's API
  if (wallet?.switchChain) {
    try {
      await wallet.switchChain(chainId);
      return;
    } catch (err) {
      // If Privy doesn't recognise the chain it throws — fall through to RPC method
      console.warn("wallet.switchChain failed, trying RPC method:", err.message);
    }
  }

  // Injected wallet (MetaMask, etc.) — use EIP-3326 / EIP-3085
  const provider = window.privyEthersProvider;
  if (!provider) throw new Error("No wallet provider available.");

  try {
    await provider.send("wallet_switchEthereumChain", [{ chainId: chainHex }]);
  } catch (switchErr) {
    // 4902 = chain not added yet; -32603 = some wallets use this instead
    if (switchErr.code === 4902 || switchErr.code === -32603) {
      await provider.send("wallet_addEthereumChain", [{
        chainId: chainHex,
        chainName: "Mantle Sepolia",
        nativeCurrency: { name: "MNT", symbol: "MNT", decimals: 18 },
        rpcUrls: ["https://rpc.sepolia.mantle.xyz"],
        blockExplorerUrls: ["https://explorer.sepolia.mantle.xyz"],
      }]);
    } else {
      throw switchErr;
    }
  }
}
