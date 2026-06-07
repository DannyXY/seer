import { createSmartAccountClient } from 'permissionless';
import { createPimlicoClient } from 'permissionless/clients/pimlico';
import { http } from 'viem';

const PIMLICO_API_KEY = import.meta.env.VITE_PIMLICO_API_KEY;

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
    if (!PIMLICO_API_KEY || PIMLICO_API_KEY === 'your_pimlico_api_key_here') {
      console.warn('VITE_PIMLICO_API_KEY not configured, smart account sponsorship disabled');
      return null;
    }

    const pimlicoUrl = `https://api.pimlico.io/v2/mantle/rpc?apikey=${PIMLICO_API_KEY}`;

    const pimlicoClient = createPimlicoClient({
      transport: http(pimlicoUrl),
      chain: mantleSepolia,
    });

    const smartAccountClient = await createSmartAccountClient({
      account: privySigner,
      chain: mantleSepolia,
      transport: http('https://rpc.sepolia.mantle.xyz'),
      bundlerTransport: http(pimlicoUrl),
      paymaster: pimlicoClient,
    });

    console.log('Smart account created:', smartAccountClient.account?.address);
    return smartAccountClient;
  } catch (error) {
    console.error('Error creating smart account:', error);
    return null;
  }
}

export async function getSmartAccountAddress(smartAccount) {
  try {
    return smartAccount?.account?.address ?? null;
  } catch (error) {
    console.error('Error getting smart account address:', error);
    return null;
  }
}
