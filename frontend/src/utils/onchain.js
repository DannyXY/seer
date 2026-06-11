/**
 * Wallet provider resolution + transaction sending for backend-prepared calldata.
 * Supports both injected wallets (MetaMask) and Privy embedded wallets (Google OAuth).
 */

const PROVIDER_WAIT_MS = 8000;

/**
 * Resolve an ethers provider for the connected wallet, waiting briefly if
 * Privy is still initializing. Order of preference:
 *   1. cached window.privyEthersProvider
 *   2. window.privyWallet.getEthersProvider() (retried while Privy boots)
 *   3. injected window.ethereum as a last resort
 * Throws a actionable error only after the wait window expires.
 */
export async function getWalletProvider({ waitMs = PROVIDER_WAIT_MS } = {}) {
  const deadline = Date.now() + waitMs;
  for (;;) {
    if (window.privyEthersProvider) return window.privyEthersProvider;
    const wallet = window.privyWallet;
    if (wallet?.getEthersProvider) {
      try {
        const provider = await wallet.getEthersProvider();
        window.privyEthersProvider = provider;
        return provider;
      } catch (_) {
        // wallet still initializing - keep waiting
      }
    }
    if (Date.now() >= deadline) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  if (window.ethereum?.request) {
    const { BrowserProvider } = await import("ethers");
    return new BrowserProvider(window.ethereum);
  }
  throw new Error("Your wallet is still connecting. Wait a few seconds and try again, or log out and back in.");
}

/**
 * Send a backend-prepared calldata object using the resolved wallet provider.
 * Auto-switches to the correct chain before sending.
 */
export async function sendOnChainTx(calldata) {
  const { to, data, chain_id, value } = calldata;
  let provider = await getWalletProvider();

  if (chain_id) {
    const network = await provider.getNetwork();
    // chainId is a number on ethers v5 providers and a bigint on v6
    if (Number(network.chainId) !== Number(chain_id)) {
      await switchToChain(chain_id, provider);
      // Always get a fresh provider after a chain switch
      if (window.privyWallet?.getEthersProvider) {
        provider = await window.privyWallet.getEthersProvider();
        window.privyEthersProvider = provider;
      }
    }
  }

  const signer = await provider.getSigner();
  const txRequest = { to, data };
  if (value && value !== "0" && value !== "0x0") txRequest.value = value;
  const tx = await signer.sendTransaction(txRequest);
  return tx.hash;
}

/**
 * Switch to the given chainId (number).
 * Privy embedded wallets use wallet.switchChain(); injected wallets use
 * wallet_switchEthereumChain / wallet_addEthereumChain.
 */
async function switchToChain(chainId, provider) {
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
