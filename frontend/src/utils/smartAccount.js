import { createSmartAccountClient, ENTRYPOINT_ADDRESS_V07 } from 'permissionless';
import { createPimlicoPaymasterClient } from 'permissionless/clients/pimlico';
import { createPimlicoClient } from 'permissionless/clients/pimlico';
import { http, publicActions } from 'viem';
import { mantle } from 'viem/chains';

const MANTLE_SEPOLIA_RPC = 'https://rpc.sepolia.mantle.xyz';
const PIMLICO_API_KEY = import.meta.env.VITE_PIMLICO_API_KEY;

export async function createSmartAccount(privyWallet) {
  try {
    if (!PIMLICO_API_KEY) {
      console.warn('VITE_PIMLICO_API_KEY not configured, skipping smart account creation');
      return null;
    }

    // Create Pimlico bundler client
    const pimlicoClient = createPimlicoClient({
      transport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      entryPoint: ENTRYPOINT_ADDRESS_V07,
    });

    // Create Pimlico paymaster client for gas sponsorship
    const paymasterClient = createPimlicoPaymasterClient({
      transport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      entryPoint: ENTRYPOINT_ADDRESS_V07,
    });

    // Create smart account client with the Privy signer
    const smartAccountClient = await createSmartAccountClient({
      account: privyWallet, // Privy's embedded wallet/signer
      chain: mantle, // Use Mantle mainnet or Sepolia testnet
      transport: http(MANTLE_SEPOLIA_RPC),
      bundlerTransport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      middleware: {
        gasPrice: async () => {
          const gasPrice = await paymasterClient.getUserOperationGasPrice();
          return gasPrice;
        },
        sponsorUserOperation: paymasterClient.sponsorUserOperation,
      },
    });

    console.log('Smart account client created successfully');
    return smartAccountClient;
  } catch (error) {
    console.error('Error creating smart account:', error);
    throw error;
  }
}

export async function deploySmartAccount(smartAccount) {
  try {
    // Smart account is automatically deployed on first user operation
    // No explicit deployment needed with Pimlico
    console.log('Smart account ready for use');
    return { success: true };
  } catch (error) {
    console.error('Error deploying smart account:', error);
    throw error;
  }
}

// Helper to get smart account address
export async function getSmartAccountAddress(smartAccount) {
  try {
    if (smartAccount.account && smartAccount.account.address) {
      return smartAccount.account.address;
    }
    return null;
  } catch (error) {
    console.error('Error getting smart account address:', error);
    return null;
  }
}
