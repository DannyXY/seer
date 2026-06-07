import { createSmartAccountClient, ENTRYPOINT_ADDRESS_V07 } from 'permissionless';
import { createPimlicoPaymasterClient } from 'permissionless/clients/pimlico';
import { createPimlicoClient } from 'permissionless/clients/pimlico';
import { http } from 'viem';

const PIMLICO_API_KEY = import.meta.env.VITE_PIMLICO_API_KEY;

// Mantle Sepolia testnet configuration
const mantleSepolia = {
  id: 5003,
  name: 'Mantle Sepolia',
  nativeCurrency: { name: 'Mantle', symbol: 'MNT', decimals: 18 },
  rpcUrls: {
    default: { http: ['https://rpc.sepolia.mantle.xyz'] },
    public: { http: ['https://rpc.sepolia.mantle.xyz'] },
  },
  blockExplorers: {
    default: { name: 'ManitaScope', url: 'https://sepolia.mantlescan.io' },
  },
};

export async function createSmartAccount(privySigner) {
  try {
    if (!PIMLICO_API_KEY) {
      console.warn('VITE_PIMLICO_API_KEY not configured, smart account sponsorship disabled');
      return null;
    }

    console.log('Creating smart account with Pimlico...');

    // Create Pimlico bundler client for sending user operations
    const pimlicoClient = createPimlicoClient({
      transport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      entryPoint: ENTRYPOINT_ADDRESS_V07,
    });

    // Create Pimlico paymaster client for sponsoring gas
    const paymasterClient = createPimlicoPaymasterClient({
      transport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      entryPoint: ENTRYPOINT_ADDRESS_V07,
    });

    // Create smart account client using Privy's signer
    const smartAccountClient = await createSmartAccountClient({
      account: privySigner,
      chain: mantleSepolia,
      transport: http('https://rpc.sepolia.mantle.xyz'),
      bundlerTransport: http(`https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`),
      entryPoint: ENTRYPOINT_ADDRESS_V07,
      middleware: {
        sponsorUserOperation: paymasterClient.sponsorUserOperation,
        gasPrice: async () => paymasterClient.getUserOperationGasPrice(),
      },
    });

    console.log('Smart account created:', smartAccountClient.account?.address);
    return smartAccountClient;
  } catch (error) {
    console.error('Error creating smart account:', error);
    // Return null instead of throwing to allow app to continue without sponsorship
    return null;
  }
}

export async function getSmartAccountAddress(smartAccount) {
  try {
    if (smartAccount?.account?.address) {
      return smartAccount.account.address;
    }
    return null;
  } catch (error) {
    console.error('Error getting smart account address:', error);
    return null;
  }
}
