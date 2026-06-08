/**
 * Send a backend-prepared calldata object using the Privy wallet provider.
 * Auto-switches to the correct chain before sending.
 */
export async function sendOnChainTx(calldata) {
  const { to, data, chain_id } = calldata;
  if (!window.privyEthersProvider) throw new Error("No wallet provider available.");
  let provider = window.privyEthersProvider;

  if (chain_id) {
    const network = await provider.getNetwork();
    if (network.chainId !== chain_id) {
      const chainHex = "0x" + chain_id.toString(16);
      try {
        await provider.send("wallet_switchEthereumChain", [{ chainId: chainHex }]);
      } catch (switchErr) {
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
      // After chain switch the old provider is invalidated — get a fresh one
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
