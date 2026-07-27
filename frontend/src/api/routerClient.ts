const ROUTER_BASE = import.meta.env.VITE_ROUTER_URL || 'http://localhost:4000';

interface FetchOpts {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
}

async function req<T>(path: string, opts: FetchOpts = {}): Promise<T> {
  const url = `${ROUTER_BASE}${path}`;
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...opts.headers,
  };
  const res = await fetch(url, {
    method: opts.method || 'GET',
    headers,
    body: opts.body ? JSON.stringify(opts.body) : undefined,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => 'Unknown error');
    throw new Error(`Router ${res.status}: ${text.slice(0, 200)}`);
  }
  return res.json();
}

// ─── Health ────────────────────────────────────────────────────────────────
export interface RouterHealth {
  status: string;
  service: string;
  uptime: number;
  timestamp: string;
}

export function getRouterHealth(): Promise<RouterHealth> {
  return req<RouterHealth>('/health');
}

// ─── Stats ─────────────────────────────────────────────────────────────────
export interface ProviderStat {
  name: string;
  healthy: boolean;
  totalRequests?: number;
  totalErrors?: number;
  lastError?: string | null;
  // Blackbox
  currentUserId?: string;
  userIdIndex?: number;
  totalUserIds?: number;
  failCount?: number;
  // Sixth
  poolSize?: number;
  activeAccounts?: number;
  currentAccount?: string | null;
  // Qveris
  hasApiKey?: boolean;
  remainingCredits?: number | null;
  modelCount?: number;
}

export interface RouterStats {
  router: {
    requestCount: number;
    successCount: number;
    failoverCount: number;
    successRate: string;
  };
  providers: {
    blackbox: ProviderStat;
    sixth: ProviderStat;
    qveris: ProviderStat;
  };
}

export function getRouterStats(): Promise<RouterStats> {
  return req<RouterStats>('/stats');
}

// ─── Models ────────────────────────────────────────────────────────────────
export interface RouterModel {
  id: string;
  object: 'model';
  created: number;
  owned_by: string;
  display_name: string;
  description: string;
  provider: 'auto' | 'blackbox' | 'sixth' | 'qveris';
}

export interface RouterModelsResponse {
  object: 'list';
  data: RouterModel[];
}

export function getRouterModels(): Promise<RouterModelsResponse> {
  return req<RouterModelsResponse>('/v1/models');
}

// ─── Chat (non-streaming for playground) ───────────────────────────────────
export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface ChatRequestPayload {
  model: string;
  messages: ChatMessage[];
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
}

export async function sendRouterChat(payload: ChatRequestPayload, onToken?: (tok: string) => void, onMeta?: (meta: { provider: string | null; model: string | null }) => void): Promise<string> {
  const res = await fetch(`${ROUTER_BASE}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...payload, stream: true }),
  });
  if (!res.ok) {
    const t = await res.text().catch(() => '');
    throw new Error(`HTTP ${res.status}: ${t.slice(0, 160)}`);
  }
  const provider = res.headers.get('X-Culi-Provider');
  const model = res.headers.get('X-Culi-Model');
  if (onMeta) onMeta({ provider, model });
  const reader = res.body?.getReader();
  if (!reader) throw new Error('No response body');
  const dec = new TextDecoder();
  let buf = '';
  let out = '';
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    const lines = buf.split('\n');
    buf = lines.pop() || '';
    for (const raw of lines) {
      const line = raw.trim();
      if (!line || !line.startsWith('data: ')) continue;
      const payload = line.slice(6);
      if (payload === '[DONE]') continue;
      try {
        const parsed = JSON.parse(payload);
        const choices = parsed.choices || [];
        for (const ch of choices) {
          const tok = ch.delta?.content;
          if (tok) {
            out += tok;
            if (onToken) onToken(tok);
          }
        }
      } catch {}
    }
  }
  return out;
}
