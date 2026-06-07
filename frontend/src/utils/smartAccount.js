import { createSmartAccountClient } from '@pimlico/permissionless';
import { http } from 'viem';

const MANTLE_SEPOLIA_RPC = 'https://rpc.sepolia.mantle.xyz';
const PIMLICO_API_KEY = import.meta.env.VITE_PIMLICO_API_KEY;

export async function createSmartAccount(signer) {
  try {
    if (!PIMLICO_API_KEY) {
      console.warn('VITE_PIMLICO_API_KEY not configured, skipping smart account creation');
      return null;
    }

    const client = await createSmartAccountClient({
      account: signer,
      chain: 5003, // Mantle Sepolia
      bundlerTransport: http(
        `https://api.pimlico.io/v1/sepolia-mantle/rpc?apikey=${PIMLICO_API_KEY}`
      ),
      middleware: {
        sponsorUserOperation: async ({ userOperation }) => {
          const response = await fetch(
            `https://api.pimlico.io/v2/sepolia-mantle/rpc?apikey=${PIMLICO_API_KEY}`,
            {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                jsonrpc: '2.0',
                method: 'pimlico_sponsorUserOperation',
                params: [userOperation, { entryPoint: '0x5FF137D4b0FDCD49DcA30c7B8b3D6f4d02F8D6c0' }],
                id: 1,
              }),
            }
          );
          const result = await response.json();
          if (result.error) throw new Error(result.error.message);
          return result.result;
        },
      },
    });

    return client;
  } catch (error) {
    console.error('Error creating smart account:', error);
    throw error;
  }
}

export async function deploySmartAccount(smartAccount) {
  try {
    const tx = await smartAccount.sendTransaction({
      to: smartAccount.account.address,
      value: 0n,
      data: '0x',
    });
    return tx;
  } catch (error) {
    console.error('Error deploying smart account:', error);
    throw error;
  }
}
