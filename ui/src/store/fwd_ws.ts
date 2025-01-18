import { create } from 'zustand'
import { ConnectionType, ProcessState } from '../types/FwdWs'

export interface FwdWsStore {
  state: ProcessState
  updateState: (state: ProcessState) => void
  setPartner: (partner: string | null) => Promise<void>
  connectToServer: (url: string) => Promise<void>
  acceptClients: (endpoint: string) => Promise<void>
  disconnect: () => Promise<void>
  refreshState: () => Promise<void>
}

const BASE_URL = import.meta.env.BASE_URL;

const useFwdWsStore = create<FwdWsStore>()((set) => ({
  state: {
    partner: null,
    connection: ConnectionType.None,
    wsUrl: null
  },
  
  updateState: (state: ProcessState) => set({ state }),
  
  setPartner: async (partner: string | null) => {
    const response = await fetch(`${BASE_URL}/api`, {
      method: 'PUT',
      body: JSON.stringify({ SetPartner: partner })
    });
    if (!response.ok) throw new Error('Failed to set partner');
    await useFwdWsStore.getState().refreshState();
  },
  
  connectToServer: async (url: string) => {
    const response = await fetch(`${BASE_URL}/api`, {
      method: 'PUT',
      body: JSON.stringify({ ConnectToServer: url })
    });
    if (!response.ok) throw new Error('Failed to connect to server');
    await useFwdWsStore.getState().refreshState();
  },
  
  acceptClients: async (endpoint: string) => {
    const response = await fetch(`${BASE_URL}/api`, {
      method: 'PUT', 
      body: JSON.stringify({ AcceptClients: endpoint })
    });
    if (!response.ok) throw new Error('Failed to start accepting clients');
    await useFwdWsStore.getState().refreshState();
  },
  
  disconnect: async () => {
    const response = await fetch(`${BASE_URL}/api`, {
      method: 'PUT',
      body: JSON.stringify({ Disconnect: null })
    });
    if (!response.ok) throw new Error('Failed to disconnect');
    await useFwdWsStore.getState().refreshState();
  },
  
  refreshState: async () => {
    const response = await fetch(`${BASE_URL}/api`);
    if (!response.ok) throw new Error('Failed to fetch state');
    const state = await response.json();
    set({ state });
  }
}));

export default useFwdWsStore
