import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { getRouterStats } from '../../api/routerClient';

// ─── Types ─────────────────────────────────────────────────────────────────
export type RouterTab = 'dashboard' | 'freemodel' | 'qveris' | 'engineer' | 'custom' | 'models' | 'playground';

export interface ApiKeyEntry {
  id: string;
  label: string;
  key: string;
  credits?: number | null;
  usedTokens?: number;
  requests?: number;
  active: boolean;
  addedAt: string;
  lastUsed?: string;
}

export interface CustomProvider {
  id: string;
  providerKey: string; // e.g. 'openai', 'anthropic', 'custom_xxx'
  displayName: string;
  baseUrl: string;
  logo?: string;
  keys: ApiKeyEntry[];
  models: string[];
  active: boolean;
  createdAt: string;
}

export interface RouterState {
  // UI
  activeTab: RouterTab;
  setActiveTab: (t: RouterTab) => void;

  // Qveris keys
  qverisKeys: ApiKeyEntry[];
  addQverisKey: (label: string, key: string) => void;
  removeQverisKey: (id: string) => void;
  toggleQverisKey: (id: string) => void;
  updateQverisCredits: (id: string, credits: number | null) => void;

  qverisRotationPolicy: {
    onRateLimit:    boolean;
    onOutOfCredits: boolean;
    onAuthError:    boolean;
    roundRobin:     boolean;
    minCreditsUsd:  number;
  };
  setQverisRotationPolicy: (p: Partial<RouterState['qverisRotationPolicy']>) => void;

  // Custom providers
  customProviders: CustomProvider[];
  addCustomProvider: (p: Omit<CustomProvider, 'id' | 'createdAt'>) => void;
  removeCustomProvider: (id: string) => void;
  updateCustomProvider: (id: string, patch: Partial<CustomProvider>) => void;
  addKeyToProvider: (providerId: string, label: string, key: string) => void;
  removeKeyFromProvider: (providerId: string, keyId: string) => void;
  toggleProviderKey: (providerId: string, keyId: string) => void;

  // Totals tracking (synced from real /stats)
  totals: {
    totalRequests: number;
    totalTokens: number;
    totalFailovers: number;
    lastUpdated: string | null;
  };
  bumpRequest: (tokens?: number, failover?: boolean) => void;
  setTotals: (t: Partial<RouterState['totals']>) => void;
  syncFromRouter: () => Promise<void>; // fetch real stats from :4000
}

function uid() { return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`; }

const defaultQveris: ApiKeyEntry[] = [];

// Seed with 4 starter custom providers (empty keys, templates)
function seedCustom(): CustomProvider[] {
  const now = new Date().toISOString();
  return [
    {
      id: uid(), providerKey: 'openai', displayName: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1', logo: '/src/assets/providers/openai.svg',
      keys: [], models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4.1'],
      active: true, createdAt: now,
    },
    {
      id: uid(), providerKey: 'anthropic', displayName: 'Anthropic Claude',
      baseUrl: 'https://api.anthropic.com/v1', logo: '/src/assets/providers/anthropic.svg',
      keys: [], models: ['claude-3-5-sonnet-20241022', 'claude-3-opus-20240229'],
      active: true, createdAt: now,
    },
    {
      id: uid(), providerKey: 'google', displayName: 'Google Gemini',
      baseUrl: 'https://generativelanguage.googleapis.com/v1beta', logo: '/src/assets/providers/gemini.svg',
      keys: [], models: ['gemini-2.5-pro-preview', 'gemini-2.5-flash'],
      active: true, createdAt: now,
    },
    {
      id: uid(), providerKey: 'groq', displayName: 'Groq',
      baseUrl: 'https://api.groq.com/openai/v1', logo: '/src/assets/providers/groq.svg',
      keys: [], models: ['llama-3.3-70b-versatile', 'mixtral-8x7b-32768'],
      active: true, createdAt: now,
    },
    {
      id: uid(), providerKey: 'openrouter', displayName: 'OpenRouter',
      baseUrl: 'https://openrouter.ai/api/v1', logo: '/src/assets/providers/openrouter.svg',
      keys: [], models: ['openai/gpt-4o', 'anthropic/claude-3.5-sonnet'],
      active: true, createdAt: now,
    },
  ];
}

export const useRouterStore = create<RouterState>()(
  persist(
    (set, get) => ({
      activeTab: 'dashboard',
      setActiveTab: (t) => set({ activeTab: t }),

      qverisKeys: defaultQveris,
      addQverisKey: (label, key) => set((s) => ({
        qverisKeys: [
          ...s.qverisKeys,
          { id: uid(), label: label || `Key ${s.qverisKeys.length + 1}`, key, credits: null, usedTokens: 0, requests: 0, active: true, addedAt: new Date().toISOString() },
        ],
      })),
      removeQverisKey: (id) => set((s) => ({ qverisKeys: s.qverisKeys.filter(k => k.id !== id) })),
      toggleQverisKey: (id) => set((s) => ({ qverisKeys: s.qverisKeys.map(k => k.id === id ? { ...k, active: !k.active } : k) })),
      updateQverisCredits: (id, credits) => set((s) => ({ qverisKeys: s.qverisKeys.map(k => k.id === id ? { ...k, credits } : k) })),

      qverisRotationPolicy: {
        onRateLimit:    true,
        onOutOfCredits: true,
        onAuthError:    true,
        roundRobin:     false,
        minCreditsUsd:  0.50,
      },
      setQverisRotationPolicy: (p) => set((s) => ({
        qverisRotationPolicy: { ...s.qverisRotationPolicy, ...p }
      })),

      customProviders: seedCustom(),
      addCustomProvider: (p) => set((s) => ({
        customProviders: [...s.customProviders, { ...p, id: uid(), createdAt: new Date().toISOString() }],
      })),
      removeCustomProvider: (id) => set((s) => ({ customProviders: s.customProviders.filter(p => p.id !== id) })),
      updateCustomProvider: (id, patch) => set((s) => ({
        customProviders: s.customProviders.map(p => p.id === id ? { ...p, ...patch } : p),
      })),
      addKeyToProvider: (pid, label, key) => set((s) => ({
        customProviders: s.customProviders.map(p => p.id === pid ? {
          ...p,
          keys: [...p.keys, { id: uid(), label: label || `Key ${p.keys.length + 1}`, key, credits: null, usedTokens: 0, requests: 0, active: true, addedAt: new Date().toISOString() }],
        } : p),
      })),
      removeKeyFromProvider: (pid, kid) => set((s) => ({
        customProviders: s.customProviders.map(p => p.id === pid ? { ...p, keys: p.keys.filter(k => k.id !== kid) } : p),
      })),
      toggleProviderKey: (pid, kid) => set((s) => ({
        customProviders: s.customProviders.map(p => p.id === pid ? {
          ...p, keys: p.keys.map(k => k.id === kid ? { ...k, active: !k.active } : k),
        } : p),
      })),

      totals: { totalRequests: 0, totalTokens: 0, totalFailovers: 0, lastUpdated: null },
      bumpRequest: (tokens = 0, failover = false) => set((s) => ({
        totals: {
          ...s.totals,
          totalRequests:  s.totals.totalRequests + 1,
          totalTokens:    s.totals.totalTokens + tokens,
          totalFailovers: s.totals.totalFailovers + (failover ? 1 : 0),
          lastUpdated:    new Date().toISOString(),
        },
      })),
      setTotals: (t) => set((s) => ({ totals: { ...s.totals, ...t, lastUpdated: new Date().toISOString() } })),

      // Fetch real stats from CulirouterAPI and update store
      syncFromRouter: async () => {
        try {
          const s = await getRouterStats();
          set((state) => ({
            totals: {
              ...state.totals,
              totalRequests:  s.router.requestCount,
              totalFailovers: s.router.failoverCount,
              lastUpdated:    new Date().toISOString(),
            },
          }));
        } catch {
          // Router offline — keep local counts
        }
      },
    }),
    {
      name: 'culi-router-store',
      partialize: (s) => ({
        qverisKeys: s.qverisKeys,
        customProviders: s.customProviders,
        totals: s.totals,
        qverisRotationPolicy: s.qverisRotationPolicy,
      }) as Partial<RouterState>,
    }
  )
);
