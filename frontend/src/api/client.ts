// CULI API client — connects frontend to Rust backend
// Supports both modes:
// - Browser: HTTP requests to localhost:3111
// - Desktop (Tauri): Direct IPC calls (faster, no HTTP overhead)

// Detect if running inside Tauri
const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

// Import Tauri invoke only in Tauri context
let tauriInvoke: any = null;
if (IS_TAURI) {
  // Dynamic import to avoid errors in browser mode
  import('@tauri-apps/api/core').then(module => {
    tauriInvoke = module.invoke;
  }).catch(() => {
    console.warn('Tauri API not available, falling back to HTTP');
  });
}

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3111/api';

interface FetchOptions {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
}

async function request<T>(path: string, opts: FetchOptions = {}): Promise<T> {
  const url = `${API_BASE}${path}`;
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
    throw new Error(`API ${res.status}: ${text.slice(0, 200)}`);
  }

  return res.json();
}

// --- Health ---
export interface HealthResponse {
  status: string;
  version: string;
  uptime_seconds?: number;
  memory_entries?: number;
  session_id: string | null;
  mode?: string; // 'desktop' or 'cli'
}

export async function getHealth(): Promise<HealthResponse> {
  if (IS_TAURI && tauriInvoke) {
    return tauriInvoke('get_health');
  }
  return request<HealthResponse>('/health');
}

// --- Audit Report ---
export interface AuditStats {
  total_files: number;
  total_violations: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  by_category: Record<string, number>;
}

export interface AuditResponse {
  report: unknown;
  stats: AuditStats;
  generated_at: string;
}

export async function runAudit(path?: string): Promise<AuditResponse> {
  if (IS_TAURI && tauriInvoke) {
    // Tauri command returns markdown directly
    const markdown = await (tauriInvoke as any)('run_audit', { path: path || '.' }) as string;
    return {
      report: markdown,
      stats: {
        total_files: 0,
        total_violations: 0,
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        by_category: {},
      },
      generated_at: new Date().toISOString(),
    };
  }
  return request<AuditResponse>('/audit/report', {
    method: 'POST',
    body: { path: path || undefined, format: 'json' },
  });
}

// --- Memory Stats ---
export interface MemoryStatsResponse {
  working_count: number;
  episodic_count: number;
  semantic_count: number;
  procedural_count: number;
  total_entries: number;
  dedup_skipped: number;
}

export async function getMemoryStats(): Promise<MemoryStatsResponse> {
  if (IS_TAURI && tauriInvoke) {
    return (tauriInvoke as any)('get_memory_stats') as Promise<MemoryStatsResponse>;
  }
  return request<MemoryStatsResponse>('/memory/stats');
}

// --- Chat ---
export interface ChatRequest {
  message: string;
  session_id?: string;
  model?: string;
  stream?: boolean;
}

export interface ChatResponse {
  message: string;
  session_id: string;
  tokens_used?: number;
  iterations?: number;
  tool_calls?: string[];
  provider?: string;
  model?: string;
}

export async function sendChat(req: ChatRequest): Promise<ChatResponse> {
  if (IS_TAURI && tauriInvoke) {
    // Tauri IPC call - direct Rust function invocation
    return (tauriInvoke as any)('send_chat', {
      request: {
        message: req.message,
        session_id: req.session_id || null,
      },
    }) as Promise<ChatResponse>;
  }
  // HTTP mode
  return request<ChatResponse>('/chat', { method: 'POST', body: req });
}

// --- Chat Streaming ---
export interface StreamEvent {
  type: 'thinking' | 'content' | 'tool_call' | 'tool_result' | 'done' | 'error';
  content?: string;
  tokens_used?: number;
  provider?: string;
  model?: string;
  iterations?: number;
  message?: string;
  // Tool call fields
  id?: string;
  name?: string;
  arguments?: any;
  // Tool result fields
  success?: boolean;
  data?: any;
  duration_ms?: number;
}

export async function* streamChat(req: ChatRequest): AsyncGenerator<StreamEvent> {
  const url = `${API_BASE}/chat/stream`;
  
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(req),
  });

  if (!response.ok) {
    throw new Error(`Stream failed: ${response.status}`);
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error('No response body');
  }

  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || ''; // Keep incomplete line in buffer

      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed.startsWith('data: ')) {
          const json = trimmed.substring(6);
          try {
            const event: StreamEvent = JSON.parse(json);
            yield event;
          } catch (e) {
            console.warn('Failed to parse SSE event:', json);
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

// --- Settings ---
export interface SettingsUpdate {
  agent_mode?: string;
  model?: string;
  provider?: string;
  theme?: string;
}

export async function updateSettings(settings: SettingsUpdate): Promise<{ status: string }> {
  if (IS_TAURI && tauriInvoke) {
    // Not implemented in Tauri commands yet
    return { status: 'ok' };
  }
  return request<{ status: string }>('/settings', { method: 'POST', body: settings });
}

// --- Utility: Check if running in Tauri ---
export function isTauriMode(): boolean {
  return IS_TAURI;
}
