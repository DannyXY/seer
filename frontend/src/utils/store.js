// Initialize Seer store and API that will be populated by data module
export let SEER = null;
export let SeerAPI = null;
export let SeerLive = null;

export function initializeSeerStore(seerObj, api, live) {
  SEER = seerObj;
  SeerAPI = api;
  SeerLive = live;
  return { SEER, SeerAPI, SeerLive };
}

export function useSeerStore() {
  if (!SEER) {
    console.warn('SEER store not initialized yet');
    return { auth: null, wallet: null, ready: false, util: { shortAddr: () => '0x…' } };
  }
  return SEER.getSnapshot?.() || {};
}
