import { useEffect, useRef, useState } from 'react';
import type { LucideIcon } from 'lucide-react';
import {
  AtSign, BookOpen, Check, ChevronDown, ChevronLeft, CircleDot, Clock,
  Code2, Eraser, FileCode2, FileText, FolderOpen, GitBranch, Globe,
  Hash, History, Image as ImageIcon, Menu, Mic, Minus, Moon,
  PanelRightClose, PanelRightOpen, Search, Send, Server,
  Settings2, Shield, Square, Terminal, Wand2, X, Bot, Plus, ChevronRight,
} from 'lucide-react';
import { create } from 'zustand';

import logoSvg from '../assets/logo.svg';
import { setupInAppReview } from './autheflow-review/web-review-integration';
import './autheflow-review/review-panel.css';
import '../styles/RouterAPI.css';

// ── Phase 3 Week 1 Components ─────────────────────────────────────────
import { ThinkingDisplay }   from './components/Chat/ThinkingDisplay';
import { ToolExecutionPanel } from './components/Chat/ToolExecutionPanel';
import { DagTraceTab }       from './components/Visualizer/DagTraceTab';
import { CodeReviewTab }     from './components/Visualizer/CodeReviewTab';
import { KnowledgeGraphTab } from './components/Visualizer/KnowledgeGraphTab';
import { KanbanTab }         from './components/Visualizer/KanbanTab';
import { ArchitectureTab }   from './components/Visualizer/ArchitectureTab';
import RouterAPIView         from './components/RouterAPI/RouterAPIView';
import { getRouterModels, type RouterModel } from './api/routerClient';
import { ProviderLogo } from './components/RouterAPI/providerLogos';
import { sendChat, streamChat } from './api/client';


/* ── Types ───────────────────────────────────────────────────────────── */

type ThemeMode = 'system' | 'light' | 'dark';
type AgentMode = 'vibe' | 'router';
type PanelTab = 'dag' | 'review' | 'graph' | 'kanban' | 'editor' | 'preview' | 'architecture';
type AgentStatus = 'idle' | 'working';
type SidebarView = 'workbench' | 'graph' | 'history' | 'knowledge';

// Tool execution types
type ToolCall = {
  id: string;
  name: string;
  arguments: any;
  timestamp?: number;
};

type ToolResult = {
  id: string;
  name: string;
  success: boolean;
  data: any;
  duration_ms: number;
  timestamp?: number;
};

type Message = { 
  id: number; 
  role: 'user' | 'agent'; 
  text: string; 
  time: string;
  thinking?: string;        // Reasoning process (optional)
  thinkingTokens?: number;  // Tokens used for thinking
  toolCalls?: ToolCall[];   // Tool calls in this message
  toolResults?: ToolResult[]; // Tool results in this message
};
type ChatTab = { 
  id: string; 
  title: string; 
  messages: Message[];
  draft: string;
};
type AppState = {
  theme: ThemeMode;
  sidebarOpen: boolean;
  sidebarView: SidebarView;        // ← active sidebar nav
  visualizerOpen: boolean;
  visualizerFullscreen: boolean;
  composerExpanded: boolean;
  agentMode: AgentMode;
  panelTab: PanelTab;
  messages: Message[];
  agentStatus: AgentStatus;
  settingsOpen: boolean;
  projectDir: string;              // ← selected project directory
  autoAccept: boolean;             // ← auto-accept all commands
  thinkingMode: boolean;           // ← show reasoning process
  // Multi-tab state
  chatTabs: ChatTab[];
  activeTabId: string;
  setTheme: (t: ThemeMode) => void;
  setSidebarOpen: (open: boolean) => void;
  setSidebarView: (v: SidebarView) => void;
  toggleSidebar: () => void;
  toggleVisualizer: () => void;
  toggleVisualizerFullscreen: () => void;
  toggleComposer: () => void;
  setAgentMode: (mode: AgentMode) => void;
  setPanelTab: (t: PanelTab) => void;
  addMessage: (m: Message) => void;
  setAgentStatus: (s: AgentStatus) => void;
  setSettingsOpen: (o: boolean) => void;
  setProjectDir: (dir: string) => void;
  toggleAutoAccept: () => void;
  toggleThinkingMode: () => void;
  // Tab actions
  addTab: () => void;
  closeTab: (id: string) => void;
  switchTab: (id: string) => void;
  renameTab: (id: string, title: string) => void;
  updateTabDraft: (id: string, draft: string) => void;
  addMessageToTab: (id: string, msg: Message) => void;
};

const useStore = create<AppState>((set) => {
  // Load chat tabs from localStorage on init
  const savedTabs = localStorage.getItem('culi-chat-tabs');
  const initialTabs: ChatTab[] = savedTabs 
    ? JSON.parse(savedTabs)
    : [{
        id: 'tab-1',
        title: 'Task 1',
        messages: [{
          id: 1, role: 'agent',
          text: 'I am your Autonomous AI Software Engineer. I can design architecture, write code, run tests, and deploy your app. Just tell me what to build.',
          time: '09:41',
        }],
        draft: '',
      }];
  
  const savedActiveId = localStorage.getItem('culi-active-tab-id');
  const initialActiveId = savedActiveId && initialTabs.find(t => t.id === savedActiveId)
    ? savedActiveId
    : initialTabs[0]?.id || 'tab-1';

  return {
  theme: 'system',
  agentMode: 'vibe' as AgentMode,
  sidebarOpen: true,
  sidebarView: 'workbench' as SidebarView,
  visualizerOpen: true,
  visualizerFullscreen: false,
  composerExpanded: true,
  panelTab: 'dag' as PanelTab,
  agentStatus: 'idle',
  settingsOpen: false,
  projectDir: localStorage.getItem('culi-project-dir') || '',
  autoAccept: localStorage.getItem('culi-auto-accept') === 'true',
  thinkingMode: localStorage.getItem('culi-thinking-mode') === 'true',
  messages: [{
    id: 1, role: 'agent',
    text: 'I am your Autonomous AI Software Engineer. I can design architecture, write code, run tests, and deploy your app. Just tell me what to build.',
    time: '09:41',
  }],
  // Multi-tab initial state (loaded from localStorage)
  chatTabs: initialTabs,
  activeTabId: initialActiveId,
  setTheme: (theme) => set({ theme }),
  setAgentMode: (agentMode) => set({ agentMode }),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setSidebarView: (sidebarView) => set({ sidebarView }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  toggleVisualizer: () => set((s) => ({ visualizerOpen: !s.visualizerOpen })),
  toggleVisualizerFullscreen: () => set((s) => ({ visualizerFullscreen: !s.visualizerFullscreen })),
  toggleComposer: () => set((s) => ({ composerExpanded: !s.composerExpanded })),
  setPanelTab: (panelTab) => set({ panelTab }),
  addMessage: (msg) => set((s) => ({ messages: [...s.messages, msg] })),
  setAgentStatus: (agentStatus) => set({ agentStatus }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setProjectDir: (dir) => {
    localStorage.setItem('culi-project-dir', dir);
    set({ projectDir: dir });
  },
  toggleAutoAccept: () => set((s) => {
    const next = !s.autoAccept;
    localStorage.setItem('culi-auto-accept', String(next));
    return { autoAccept: next };
  }),
  toggleThinkingMode: () => set((s) => {
    const next = !s.thinkingMode;
    localStorage.setItem('culi-thinking-mode', String(next));
    return { thinkingMode: next };
  }),
  // Tab actions
  addTab: () => set((s) => {
    // Max 6 tasks limit
    if (s.chatTabs.length >= 6) return s;
    
    const newId = `tab-${Date.now()}`;
    const newTab: ChatTab = {
      id: newId,
      title: `Task ${s.chatTabs.length + 1}`,
      messages: [{
        id: Date.now(), role: 'agent',
        text: 'I am your Autonomous AI Software Engineer. I can design architecture, write code, run tests, and deploy your app. Just tell me what to build.',
        time: fmt(),
      }],
      draft: '',
    };
    const newTabs = [...s.chatTabs, newTab];
    
    // Save to localStorage
    localStorage.setItem('culi-chat-tabs', JSON.stringify(newTabs));
    localStorage.setItem('culi-active-tab-id', newId);
    
    return { chatTabs: newTabs, activeTabId: newId };
  }),
  closeTab: (id) => set((s) => {
    if (s.chatTabs.length === 1) return s; // Don't close last tab
    const newTabs = s.chatTabs.filter(t => t.id !== id);
    const newActiveId = s.activeTabId === id ? newTabs[0].id : s.activeTabId;
    
    // Save to localStorage
    localStorage.setItem('culi-chat-tabs', JSON.stringify(newTabs));
    localStorage.setItem('culi-active-tab-id', newActiveId);
    
    return { chatTabs: newTabs, activeTabId: newActiveId };
  }),
  switchTab: (id) => {
    localStorage.setItem('culi-active-tab-id', id);
    set({ activeTabId: id });
  },
  renameTab: (id, title) => set((s) => {
    const newTabs = s.chatTabs.map(t => t.id === id ? { ...t, title: title.trim() || t.title } : t);
    localStorage.setItem('culi-chat-tabs', JSON.stringify(newTabs));
    return { chatTabs: newTabs };
  }),
  updateTabDraft: (id, draft) => set((s) => {
    const newTabs = s.chatTabs.map(t => t.id === id ? { ...t, draft } : t);
    localStorage.setItem('culi-chat-tabs', JSON.stringify(newTabs));
    return { chatTabs: newTabs };
  }),
  addMessageToTab: (id, msg) => set((s) => {
    const newTabs = s.chatTabs.map(t => t.id === id ? { ...t, messages: [...t.messages, msg] } : t);
    localStorage.setItem('culi-chat-tabs', JSON.stringify(newTabs));
    return { chatTabs: newTabs };
  }),
}});

function fmt(): string {
  return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

const SUGGESTIONS = [
  'Build a Todo App with React',
  'Refactor the authentication flow',
  'Deploy this project to Vercel',
];

/* ══════════════════════════════════════════════════════════════════════
   APP
   ══════════════════════════════════════════════════════════════════════ */

const CULI_FALLBACK_MODELS: any[] = [
  { id: 'culi-auto',   display_name: 'CULI Auto',   tier: 'auto',     description: 'Smart routing' },
  { id: 'culi-flash',  display_name: 'CULI Flash',  tier: 'fast',     description: 'Fast & efficient' },
  { id: 'culi-pro',    display_name: 'CULI Pro',    tier: 'balanced', description: 'Complex features' },
  { id: 'culi-coder',  display_name: 'CULI Coder',  tier: 'balanced', description: 'Deep reasoning' },
  { id: 'culi-ultra',  display_name: 'CULI Ultra',  tier: 'powerful', description: 'Maximum capability' },
  { id: 'culi-vision', display_name: 'CULI Vision', tier: 'balanced', description: 'Multimodal' },
];

function App() {
  const {
    theme, agentMode, sidebarOpen, sidebarView, visualizerOpen, visualizerFullscreen, composerExpanded, panelTab,
    messages, agentStatus, settingsOpen, projectDir, autoAccept,
    chatTabs, activeTabId,
    setTheme, setAgentMode, setSidebarOpen, setSidebarView, toggleSidebar, toggleVisualizer, toggleVisualizerFullscreen, toggleComposer,
    setPanelTab, addMessage, setAgentStatus, setSettingsOpen, setProjectDir, toggleAutoAccept,
    addTab, closeTab, switchTab, renameTab, updateTabDraft, addMessageToTab,
  } = useStore();

  const [mobileNav, setMobileNav] = useState(false);
  const [models, setModels] = useState<any[]>(CULI_FALLBACK_MODELS);
  const [selectedModel, setSelectedModel] = useState('culi-auto');
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // ── Backend health status ────────────────────────────────────────────
  const [backendOnline, setBackendOnline] = useState(false);
  const [tokenCount, setTokenCount] = useState(0);
  const [memoryEntries, setMemoryEntries] = useState(0);

  useEffect(() => {
    async function checkHealth() {
      try {
        const res = await fetch('http://localhost:3111/api/health');
        if (res.ok) {
          const data = await res.json();
          setBackendOnline(true);
          setMemoryEntries(data.memory_entries ?? 0);
        }
      } catch { setBackendOnline(false); }
    }
    checkHealth();
    const id = setInterval(checkHealth, 8000);
    return () => clearInterval(id);
  }, []);

  // ── Project folder picker ────────────────────────────────────────────
  const pickProjectDir = async () => {
    // Electron environment: use IPC
    if (typeof window !== 'undefined' && (window as any).electronAPI?.pickFolder) {
      const dir = await (window as any).electronAPI.pickFolder();
      if (dir) setProjectDir(dir);
      return;
    }
    // Browser / fallback: prompt for path
    const dir = window.prompt('Enter project directory path:', projectDir || 'D:\\');
    if (dir && dir.trim()) setProjectDir(dir.trim());
  };

  // ── Tauri startup state ─────────────────────────────────────────────
  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;
  const [appReady, setAppReady] = useState(!IS_TAURI); // In browser: always ready
  const [startupMsg, setStartupMsg] = useState('Starting CulirouterAPI...');

  useEffect(() => {
    if (!IS_TAURI) return;
    // Listen for Tauri ready/error events from Rust backend
    let unlisten: (() => void) | null = null;
    let unlistenErr: (() => void) | null = null;

    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{ router: boolean }>('culi://ready', (e) => {
        setStartupMsg(e.payload?.router ? 'CULI ready ✓' : 'CULI ready (router offline)');
        setTimeout(() => setAppReady(true), 400);
      }).then(fn => { unlisten = fn; });

      listen<string>('culi://error', (e) => {
        setStartupMsg(`Error: ${e.payload}`);
        setTimeout(() => setAppReady(true), 2000); // Show error briefly then open
      }).then(fn => { unlistenErr = fn; });
    });

    // Timeout fallback: open app after 15s regardless
    const fallback = setTimeout(() => setAppReady(true), 15000);
    return () => {
      unlisten?.();
      unlistenErr?.();
      clearTimeout(fallback);
    };
  }, [IS_TAURI]);

  useEffect(() => {
    let active = true;
    // Load both CULI models and Router models
    Promise.all([
      fetch('http://localhost:3111/api/models').then(r => r.json()).catch(() => ({ models: [] })),
      getRouterModels().catch(() => [])
    ]).then(([backendData, routerModels]) => {
      if (!active) return;
      const culiModels = backendData.models || CULI_FALLBACK_MODELS;
      const routerArray = Array.isArray(routerModels) ? routerModels : [];
      const allModels = [...culiModels, ...routerArray];
      setModels(allModels);
    });
    return () => { active = false; };
  }, []);
  
  // Tab editing state
  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState('');
  
  // Get active tab
  const activeTab = chatTabs.find(t => t.id === activeTabId) || chatTabs[0];
  const draft = activeTab.draft;
  const setDraft = (value: string) => updateTabDraft(activeTab.id, value);
  
  // Resizer state
  const [vizWidth, setVizWidth] = useState<number>(() => {
    const saved = localStorage.getItem('culi-viz-width');
    return saved ? parseInt(saved, 10) : 340;
  });
  const [isResizing, setIsResizing] = useState(false);
  const resizerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') {
      root.dataset.theme = window.matchMedia('(prefers-color-scheme:light)').matches ? 'specimen' : 'midnight';
    } else {
      root.dataset.theme = theme === 'dark' ? 'midnight' : 'specimen';
    }
  }, [theme]);

  // Update CSS variable when vizWidth changes
  useEffect(() => {
    document.documentElement.style.setProperty('--viz-w', `${vizWidth}px`);
    localStorage.setItem('culi-viz-width', vizWidth.toString());
  }, [vizWidth]);

  // Resizer drag logic
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing) return;
      const newWidth = window.innerWidth - e.clientX;
      // Clamp between 280px and 800px
      const clampedWidth = Math.max(280, Math.min(800, newWidth));
      setVizWidth(clampedWidth);
    };

    const handleMouseUp = () => {
      if (isResizing) {
        setIsResizing(false);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }
    };

    if (isResizing) {
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isResizing]);

  // Auto-expand composer textarea based on content
  useEffect(() => {
    if (inputRef.current) {
      const textarea = inputRef.current;
      // Reset height to auto to get proper scrollHeight
      textarea.style.height = 'auto';
      // Calculate new height (min: 66px, max: 100px)
      const newHeight = Math.min(Math.max(textarea.scrollHeight, 66), 100);
      textarea.style.height = `${newHeight}px`;
    }
  }, [draft]);

  useEffect(() => {
    setupInAppReview();
  }, []);

  // Persist chatTabs to localStorage
  useEffect(() => {
    const saved = localStorage.getItem('culi-chat-tabs');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          useStore.setState({ chatTabs: parsed, activeTabId: parsed[0].id });
        }
      } catch (e) {
        console.warn('Failed to restore chat tabs:', e);
      }
    }
  }, []);

  useEffect(() => {
    localStorage.setItem('culi-chat-tabs', JSON.stringify(chatTabs));
  }, [chatTabs]);

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)');
    if (mq.matches && visualizerOpen) toggleVisualizer();
    const h = () => { if (mq.matches && useStore.getState().visualizerOpen) useStore.getState().toggleVisualizer(); };
    mq.addEventListener('change', h);
    return () => mq.removeEventListener('change', h);
  }, [toggleVisualizer]);

  useEffect(() => {
    if (agentStatus === 'working') {
      const t = setTimeout(() => setAgentStatus('idle'), 3000);
      return () => clearTimeout(t);
    }
  }, [agentStatus, setAgentStatus]);

  const send = async () => {
    const text = draft.trim();
    if (!text) return;
    const sessionId = activeTab.id;
    addMessageToTab(activeTab.id, { id: Date.now(), role: 'user', text, time: fmt() });
    setDraft('');
    setAgentStatus('working');

    // Build context — include project dir and auto-accept flag
    const context = [
      projectDir ? `Working directory: ${projectDir}` : null,
      autoAccept ? 'Auto-accept mode: ON (execute all commands without confirmation)' : null,
    ].filter(Boolean).join('\n');

    const fullMessage = context ? `${context}\n\n${text}` : text;

    // Prepare agent message placeholders
    const agentMsgId = Date.now() + 1;
    let thinkingContent = '';
    let answerContent = '';

    // Add initial thinking message
    addMessageToTab(activeTab.id, {
      id: agentMsgId,
      role: 'agent',
      text: '',
      time: fmt(),
      thinking: '',
    });

    try {
      // Stream response
      for await (const event of streamChat({
        message: fullMessage,
        session_id: sessionId,
        model: selectedModel,
      })) {
        switch (event.type) {
          case 'thinking':
            thinkingContent += event.content || '';
            // Update message with thinking content
            useStore.setState(s => ({
              chatTabs: s.chatTabs.map(t =>
                t.id === activeTab.id
                  ? {
                      ...t,
                      messages: t.messages.map(m =>
                        m.id === agentMsgId
                          ? { ...m, thinking: thinkingContent }
                          : m
                      ),
                    }
                  : t
              ),
            }));
            break;

          case 'content':
            answerContent += event.content || '';
            // Update message with answer content
            useStore.setState(s => ({
              chatTabs: s.chatTabs.map(t =>
                t.id === activeTab.id
                  ? {
                      ...t,
                      messages: t.messages.map(m =>
                        m.id === agentMsgId
                          ? { ...m, text: answerContent }
                          : m
                      ),
                    }
                  : t
              ),
            }));
            break;

          case 'tool_call':
            // Add tool call to current message
            if (event.id && event.name) {
              useStore.setState(s => ({
                chatTabs: s.chatTabs.map(t =>
                  t.id === activeTab.id
                    ? {
                        ...t,
                        messages: t.messages.map(m =>
                          m.id === agentMsgId
                            ? {
                                ...m,
                                toolCalls: [
                                  ...(m.toolCalls || []),
                                  {
                                    id: event.id!,
                                    name: event.name!,
                                    arguments: event.arguments || {},
                                    timestamp: Date.now(),
                                  },
                                ],
                              }
                            : m
                        ),
                      }
                    : t
                ),
              }));
            }
            break;

          case 'tool_result':
            // Add tool result to current message
            if (event.id && event.name && event.success !== undefined && event.duration_ms !== undefined) {
              useStore.setState(s => ({
                chatTabs: s.chatTabs.map(t =>
                  t.id === activeTab.id
                    ? {
                        ...t,
                        messages: t.messages.map(m =>
                          m.id === agentMsgId
                            ? {
                                ...m,
                                toolResults: [
                                  ...(m.toolResults || []),
                                  {
                                    id: event.id!,
                                    name: event.name!,
                                    success: event.success!,
                                    data: event.data || null,
                                    duration_ms: event.duration_ms!,
                                    timestamp: Date.now(),
                                  },
                                ],
                              }
                            : m
                        ),
                      }
                    : t
                ),
              }));
            }
            break;

          case 'done':
            setTokenCount(t => t + (event.tokens_used ?? 0));
            // Save to localStorage
            localStorage.setItem('culi-chat-tabs', JSON.stringify(useStore.getState().chatTabs));
            break;

          case 'error':
            answerContent = `⚠️ Stream error: ${event.message}`;
            useStore.setState(s => ({
              chatTabs: s.chatTabs.map(t =>
                t.id === activeTab.id
                  ? {
                      ...t,
                      messages: t.messages.map(m =>
                        m.id === agentMsgId
                          ? { ...m, text: answerContent }
                          : m
                      ),
                    }
                  : t
              ),
            }));
            break;
        }
      }
    } catch (err: any) {
      // Fallback to non-streaming on error
      console.warn('Streaming failed, falling back to non-streaming:', err);
      
      try {
        const res = await sendChat({
          message: fullMessage,
          session_id: sessionId,
          model: selectedModel,
        });
        
        setTokenCount(t => t + (res.tokens_used ?? 0));
        useStore.setState(s => ({
          chatTabs: s.chatTabs.map(t =>
            t.id === activeTab.id
              ? {
                  ...t,
                  messages: t.messages.map(m =>
                    m.id === agentMsgId
                      ? { ...m, text: res.message }
                      : m
                  ),
                }
              : t
          ),
        }));
      } catch (fallbackErr: any) {
        useStore.setState(s => ({
          chatTabs: s.chatTabs.map(t =>
            t.id === activeTab.id
              ? {
                  ...t,
                  messages: t.messages.map(m =>
                    m.id === agentMsgId
                      ? {
                          ...m,
                          text: `⚠️ Lỗi kết nối backend: ${fallbackErr.message}. Hãy chắc chắn CULI server đang chạy.`,
                        }
                      : m
                  ),
                }
              : t
          ),
        }));
      }
    } finally {
      setAgentStatus('idle');
    }

    inputRef.current?.focus();
  };

  useEffect(() => {
    if (visualizerFullscreen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [visualizerFullscreen]);

  return (
    <>
    {/* ── TAURI LOADING SCREEN ─────────────────────────────── */}
    {!appReady && (
      <div style={{
        position: 'fixed', inset: 0, zIndex: 9999,
        background: 'var(--color-paper-1, #0d0f14)',
        display: 'flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center', gap: 24,
      }}>
        <img src={logoSvg} alt="CULI" style={{ width: 72, height: 72, opacity: 0.9 }} />
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 22, fontWeight: 700, color: 'var(--color-ink, #e8eaf0)', marginBottom: 8 }}>
            CULI Agent
          </div>
          <div style={{ fontSize: 13, color: 'var(--color-muted, #666)', fontFamily: 'monospace' }}>
            {startupMsg}
          </div>
        </div>
        {/* Spinner */}
        <div style={{
          width: 32, height: 32, borderRadius: '50%',
          border: '3px solid #2aa96730',
          borderTop: '3px solid #2aa967',
          animation: 'spin 0.8s linear infinite',
        }} />
        <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
      </div>
    )}

    <div className={`app-shell${sidebarOpen ? '' : ' sc'}${visualizerOpen ? '' : ' viz-collapsed'}${visualizerFullscreen ? ' vf' : ''}`}
         style={{ opacity: appReady ? 1 : 0, transition: 'opacity 0.3s' }}>

      {/* ── TOP BAR ──────────────────────────────────────────── */}
      <header className="topbar">
        <button className="icon-btn mobile-only" aria-label="Open explorer" onClick={() => { setSidebarOpen(true); setMobileNav(true); }}>
          <Menu size={18} />
        </button>
        <div className="logo">
          <img src={logoSvg} alt="" className="logo-img" />
          <span className="logo-text">CULI <span className="accent">AGENT</span></span>
        </div>
        <div className="workspace" onClick={pickProjectDir} title="Click to open project folder" style={{ cursor: 'pointer' }}>
          <FolderOpen size={13} />
          <span>{projectDir ? projectDir.split(/[\\/]/).pop() || projectDir : 'Open Project…'}</span>
        </div>
        <div className="mode-slider">
          <button
            className={`mode-btn${agentMode === 'vibe' ? ' active' : ''}`}
            onClick={() => setAgentMode('vibe')}
          >
            <Terminal size={12} />
            <span>VIBE</span>
          </button>
          <button
            className={`mode-btn${agentMode === 'router' ? ' active' : ''}`}
            onClick={() => setAgentMode('router')}
          >
            <Globe size={12} />
            <span>ROUTER API</span>
          </button>
        </div>
        <div className="top-act">
          <button className="icon-btn kb" onClick={() => setSettingsOpen(true)}><BookOpen size={15} /><span className="kb-lbl">Knowledge Base</span></button>
          <button className="icon-btn" onClick={() => setSettingsOpen(true)}><Settings2 size={15} /></button>
          <button className="icon-btn" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}><Moon size={14} /></button>
          <div className="win-ctrls">
            <button className="win-btn" title="Minimize"><Minus size={11} /></button>
            <button className="win-btn" title="Maximize"><Square size={9} /></button>
            <button className="win-btn" title="Close"><X size={13} /></button>
          </div>
        </div>
      </header>

      {/* ── SIDEBAR ─────────────────────────────────────────────── */}
      <aside className={`sidebar${mobileNav ? ' open' : ''}`}>
        <div className="s-hdr">
          <span className="eyebrow">Explorer</span>
          <div className="s-hdr-act">
            <button className="icon-btn" aria-label="Close explorer" onClick={() => { setSidebarOpen(false); setMobileNav(false); }}><ChevronLeft size={13} /></button>
          </div>
        </div>
        <div className="s-search">
          <Search size={12} className="s-search-ic" />
          <input type="text" placeholder="Search files…" className="s-search-in" />
        </div>
        <nav className="s-nav">
          <NavItem icon={Terminal}  label="Workbench"   active={sidebarView === 'workbench'}
            onClick={() => { setSidebarView('workbench'); setAgentMode('vibe'); }} />
          <NavItem icon={GitBranch} label="Agent Graph" meta={String(chatTabs.length).padStart(2,'0')}
            active={sidebarView === 'graph'}
            onClick={() => setSidebarView('graph')} />
          <NavItem icon={History}   label="History"     meta="12"
            active={sidebarView === 'history'}
            onClick={() => setSidebarView('history')} />
          <NavItem icon={BookOpen}  label="Knowledge"
            active={sidebarView === 'knowledge'}
            onClick={() => { setSidebarView('knowledge'); setAgentMode('router'); }} />
          <button
            className="n-item new-task-btn"
            onClick={addTab}
            disabled={chatTabs.length >= 6}
            title={chatTabs.length >= 6 ? 'Max 6 tasks reached' : 'Create new task'}
          >
            <span className="n-ic"><Wand2 size={16} /></span>
            <span className="n-lbl">New Task</span>
            <span className="n-plus">+</span>
          </button>
        </nav>
        <div className="s-section">
          <div className="eyebrow" style={{ marginBottom: 6 }}>ACTIVE CONTEXT</div>
          {projectDir ? (
            <>
              <div className="s-file s-file-active" onClick={pickProjectDir} title={projectDir} style={{ cursor: 'pointer' }}>
                <FolderOpen size={13} style={{ color: 'var(--color-accent)' }} />
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '140px' }}>
                  {projectDir}
                </span>
              </div>
              <div className="s-file" style={{ fontSize: '10px', color: 'var(--color-muted)', fontFamily: 'monospace' }}>
                {chatTabs.length} active task{chatTabs.length !== 1 ? 's' : ''}
              </div>
            </>
          ) : (
            <button className="s-file s-file-btn" onClick={pickProjectDir} style={{ cursor: 'pointer', background: 'none', border: '1px dashed var(--color-rule)', borderRadius: 4, padding: '4px 8px', width: '100%', color: 'var(--color-muted)', fontSize: 11 }}>
              <FolderOpen size={12} /> Select Project…
            </button>
          )}

          {/* Auto-accept toggle */}
          <div className="s-auto-accept" style={{ marginTop: 8, display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 6 }}>
            <span style={{ fontSize: 10, color: 'var(--color-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Auto-accept
            </span>
            <button
              onClick={toggleAutoAccept}
              title={autoAccept ? 'Auto-accept ON — all commands run without confirmation' : 'Auto-accept OFF'}
              style={{
                display: 'inline-flex', alignItems: 'center', gap: 4,
                padding: '2px 8px', borderRadius: 20,
                border: '1px solid',
                fontSize: 10, fontWeight: 700,
                cursor: 'pointer', transition: 'all .18s',
                background:   autoAccept ? 'var(--color-accent)' : 'var(--color-surface)',
                borderColor:  autoAccept ? 'var(--color-accent)' : 'var(--color-rule)',
                color:        autoAccept ? '#fff' : 'var(--color-muted)',
              }}
            >
              {autoAccept ? '⚡ ON' : 'OFF'}
            </button>
          </div>
        </div>
        <div className="s-bottom">
          <div className="eyebrow">APPEARANCE</div>
          <div className="theme-picker">
            <ThemeBtn value="system" current={theme} onClick={() => setTheme('system')} />
            <ThemeBtn value="light" current={theme} onClick={() => setTheme('light')} />
            <ThemeBtn value="dark" current={theme} onClick={() => setTheme('dark')} />
          </div>
        </div>
      </aside>

      {/* ── CHAT PANEL ──────────────────────────────────────────── */}
      <main className="chat-panel">
        {/* Helper Pill */}
        <div className={`pill${agentStatus === 'working' ? ' show' : ''}`}>
          <span className="pill-spin" />
          <span>CULI Helper is working…</span>
        </div>

        {/* Banner */}
        <div className="banner">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
            <span className={`banner-dot${agentStatus === 'working' ? ' busy' : ''}`} />
            <span className="banner-status">{agentStatus === 'working' ? 'Agent Working' : 'Agent Idle'}</span>
          </div>
          <div className="banner-r">
            <span className="banner-mode">{agentMode === 'router' ? 'Router API' : 'Vibe Coding'}</span>
          </div>
        </div>

        {/* Tab Bar */}
        {chatTabs.length > 1 && (
          <div className="tab-bar">
            {chatTabs.map(tab => (
              <div 
                key={tab.id}
                className={`chat-tab${tab.id === activeTabId ? ' active' : ''}`}
                onClick={() => switchTab(tab.id)}
              >
                {editingTabId === tab.id ? (
                  <input
                    type="text"
                    className="chat-tab-input"
                    value={editingTitle}
                    onChange={(e) => setEditingTitle(e.target.value)}
                    onBlur={() => {
                      renameTab(tab.id, editingTitle);
                      setEditingTabId(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        renameTab(tab.id, editingTitle);
                        setEditingTabId(null);
                      } else if (e.key === 'Escape') {
                        setEditingTabId(null);
                      }
                    }}
                    autoFocus
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <span 
                    className="chat-tab-title"
                    onDoubleClick={(e) => {
                      e.stopPropagation();
                      setEditingTabId(tab.id);
                      setEditingTitle(tab.title);
                    }}
                  >
                    {tab.title}
                  </span>
                )}
                {chatTabs.length > 1 && (
                  <button 
                    className="chat-tab-close"
                    onClick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
                    aria-label="Close tab"
                  >
                    <X size={11} />
                  </button>
                )}
              </div>
            ))}
            <button 
              className="chat-tab-add" 
              onClick={addTab} 
              disabled={chatTabs.length >= 6}
              aria-label="New tab"
              title={chatTabs.length >= 6 ? 'Max 6 tasks reached' : 'New tab'}
            >
              <span>+</span>
            </button>
          </div>
        )}

        {/* ── Dual View Container ── */}
        <div className={`view-container${agentMode === 'router' ? ' slide-router' : ' slide-vibe'}`}>
          
          {/* ── VIBE VIEW (Workbench / History / Graph) ── */}
          <div className="view view-vibe">
            {/* Sidebar view: History */}
            {sidebarView === 'history' && (
              <div className="sv-panel">
                <div className="sv-header"><History size={14} /> Conversation History</div>
                <div className="sv-list">
                  {chatTabs.map((tab, i) => (
                    <div key={tab.id} className={`sv-item${tab.id === activeTabId ? ' sv-item-active' : ''}`}
                      onClick={() => { switchTab(tab.id); setSidebarView('workbench'); }}>
                      <Terminal size={12} />
                      <div className="sv-item-info">
                        <span className="sv-item-title">{tab.title}</span>
                        <span className="sv-item-meta">{tab.messages.length} messages</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Sidebar view: Agent Graph */}
            {sidebarView === 'graph' && (
              <div className="sv-panel">
                <div className="sv-header"><GitBranch size={14} /> Agent Graph</div>
                <div className="sv-graph-nodes">
                  {[
                    { label: 'Orchestrator', sub: 'Routing & planning', role: 'architect' },
                    { label: 'Memory Pipeline', sub: 'Working · Episodic · Semantic', role: 'memory' },
                    { label: 'Tool Registry', sub: '7 tools registered', role: 'dev' },
                    { label: 'LLM Router', sub: backendOnline ? 'Blackbox → Sixth → Ollama' : 'Offline', role: backendOnline ? 'provider' : 'error' },
                  ].map(n => (
                    <div key={n.label} className={`sv-node badge-role ${n.role}`}>
                      <strong>{n.label}</strong>
                      <span>{n.sub}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Main Workbench (default) */}
            {(sidebarView === 'workbench' || sidebarView === 'knowledge') && (<>
            {/* Messages Area */}
            <div className="msgs">
              <div className="welcome">
                <div className="welcome-av">
                  <img src={logoSvg} alt="" className="welcome-logo" />
                </div>
                <div>
                  <h3 className="welcome-t">Welcome to CULI Command Center</h3>
                  <p className="welcome-d">{activeTab.messages[0].text}</p>
                  <div className="chips">
                    {SUGGESTIONS.map(s => (
                      <button key={s} className="chip" onClick={() => setDraft(s)}>{s}</button>
                    ))}
                  </div>
                </div>
              </div>
              {activeTab.messages.slice(1).map(m => (
                <article key={m.id} className={`msg ${m.role}`}>
                  <div className="msg-gut">
                    <span>{m.role === 'agent' ? 'SYS' : 'USR'}</span>
                    <time>{m.time}</time>
                  </div>
                  <div className="msg-body">
                    {m.thinking && useStore.getState().thinkingMode && (
                      <ThinkingDisplay thinking={m.thinking} />
                    )}
                    {(m.toolCalls || m.toolResults) && (
                      <ToolExecutionPanel
                        toolCalls={m.toolCalls}
                        toolResults={m.toolResults}
                      />
                    )}
                    <p>{m.text}</p>
                  </div>
                </article>
              ))}
              {/* Loading indicator when agent is working */}
              {agentStatus === 'working' && (
                <article className="msg agent msg-loading">
                  <div className="msg-gut">
                    <span>SYS</span>
                    <time>{fmt()}</time>
                  </div>
                  <div className="msg-body">
                    <div className="loading-indicator">
                      <div className="loading-spinner" />
                      <span className="loading-text">Agent đang suy nghĩ...</span>
                    </div>
                  </div>
                </article>
              )}
            </div>

            {/* Composer */}
            <div className="composer-wrap">
              <div className={`composer${composerExpanded ? '' : ' collapsed'}`}>
                <div className="c-label" onClick={toggleComposer} title={composerExpanded ? 'Collapse' : 'Expand'}>
                  <span className={`c-expand-btn${composerExpanded ? '' : ' collapsed'}`}>
                    <ChevronDown size={10} />
                  </span>
                  <Bot size={11} />
                  <span>@CULI-AGENT-2.0</span>
                  <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                    <Wand2 size={10} style={{ color: 'var(--accent)', opacity: 0.7 }} />
                  </div>
                </div>
                <textarea
                  ref={inputRef}
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => {
                    // Enter to send (Shift+Enter for newline)
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      send();
                    }
                  }}
                  placeholder="You are chatting with CULI Agent now… (Enter to send, Shift+Enter for newline)"
                  style={{ display: composerExpanded ? 'block' : 'none' }}
                />
                <div className="c-tbar">
                  <div className="c-l">
                    <ToolBtn icon={AtSign} />
                    <ToolBtn icon={Hash} />
                    <ToolBtn icon={ImageIcon} />
                  </div>
                  <ModelSelector models={models} value={selectedModel} onChange={setSelectedModel} />
                  <div className="c-r">
                    <button 
                      className={`t-btn${useStore.getState().thinkingMode ? ' active' : ''}`}
                      onClick={useStore.getState().toggleThinkingMode}
                      title={useStore.getState().thinkingMode ? 'Hide reasoning process' : 'Show reasoning process'}
                      style={{ color: useStore.getState().thinkingMode ? 'var(--color-accent)' : undefined }}
                    >
                      <Wand2 size={13} />
                    </button>
                    <ToolBtn icon={Eraser} />
                    <ToolBtn icon={Mic} />
                    <button className="send-btn" onClick={send} disabled={!draft.trim() || agentStatus === 'working'}>
                      <Send size={14} />
                    </button>
                  </div>
                </div>
              </div>
              <div className="composer-fn">Powered by CULI Intelligence • v2.0</div>
            </div>
            </>)}  {/* end workbench fragment */}
          </div>  {/* end view view-vibe */}

          {/* ── ROUTER API VIEW ── */}
          <div className="view view-router">
            <RouterAPIView />
          </div>


        </div>
      </main>

      {/* ── VISUALIZER EXPAND BUTTON ─────────────────────────── */}
      <button className="viz-expand-btn" onClick={toggleVisualizer} aria-label="Expand visualizer">
        <ChevronLeft size={14} />
      </button>

      {/* ── VISUALIZER ──────────────────────────────────────────── */}
      <aside className="viz">
        {/* Resizer - draggable divider */}
        <div
          ref={resizerRef}
          className={`viz-resizer${isResizing ? ' resizing' : ''}`}
          onMouseDown={() => setIsResizing(true)}
        />
        
        <div className="viz-bar">
          <VizTab active={panelTab === 'dag'}          onClick={() => setPanelTab('dag')}          label="DAG" />
          <VizTab active={panelTab === 'architecture'} onClick={() => setPanelTab('architecture')} label="Arch" />
          <VizTab active={panelTab === 'review'}       onClick={() => setPanelTab('review')}       label="Review" />
          <VizTab active={panelTab === 'graph'}        onClick={() => setPanelTab('graph')}        label="Graph" />
          <VizTab active={panelTab === 'kanban'}       onClick={() => setPanelTab('kanban')}       label="Kanban" />
          <VizTab active={panelTab === 'editor'}       onClick={() => setPanelTab('editor')}       label="Editor" />
          <VizTab active={panelTab === 'preview'}      onClick={() => setPanelTab('preview')}      label="Preview" />
          <button className="icon-btn" onClick={toggleVisualizerFullscreen} title="Fullscreen" style={{ marginLeft: 'auto' }}>
            {visualizerFullscreen ? <PanelRightClose size={15} /> : <Square size={13} />}
          </button>
          {!visualizerFullscreen && (
            <button className="icon-btn" onClick={toggleVisualizer} title={visualizerOpen ? 'Close panel' : 'Open panel'}>
              {visualizerOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}
            </button>
          )}
        </div>
        <div className="viz-body">
          {panelTab === 'dag'          && <DagTraceTab />}
          {panelTab === 'architecture' && <ArchitectureTab />}
          {panelTab === 'review'       && <CodeReviewTab />}
          {panelTab === 'graph'        && <KnowledgeGraphTab />}
          {panelTab === 'kanban'       && <KanbanTab />}
          {panelTab === 'editor'       && <EditorPanel />}
          {panelTab === 'preview'      && <PreviewPanel />}
        </div>
      </aside>

      {/* ── STATUS BAR ──────────────────────────────────────────── */}
      <footer className="statusbar">
        <span className="s-item" style={{ color: backendOnline ? 'var(--color-accent)' : 'var(--color-muted)' }}>
          <CircleDot size={7} />
          {backendOnline ? 'connected' : 'offline'}
        </span>
        <span className="s-sep">·</span>
        <span className="s-item"><BookOpen size={10} />{memoryEntries} mem</span>
        <span className="s-sep">·</span>
        <span className="s-item">
          <GitBranch size={11} />
          {projectDir ? (projectDir.split(/[\\/]/).pop() || 'project') : 'no project'}
        </span>
        <span className="s-sep">·</span>
        <span className="s-item"
          style={{ cursor: 'pointer', color: autoAccept ? 'var(--color-accent)' : undefined }}
          onClick={toggleAutoAccept}
          title={autoAccept ? 'Auto-accept ON — click to disable' : 'Auto-accept OFF — click to enable'}
        >
          ⚡ {autoAccept ? 'auto' : 'manual'}
        </span>
        <span className="s-sep">·</span>
        <span className="s-item">{tokenCount.toLocaleString()} tokens</span>
        <span className="s-fill" />
        <span className="s-item">CULI / v0.1.0</span>
      </footer>

      {/* ── SETTINGS MODAL ──────────────────────────────────────── */}
      <div className={`modal-over${settingsOpen ? ' open' : ''}`} onClick={() => setSettingsOpen(false)}>
        <div className="modal" onClick={(e) => e.stopPropagation()}>
          <div className="m-h">
            <div className="m-h-l"><Settings2 size={18} /><h3>Settings</h3></div>
            <button className="icon-btn" onClick={() => setSettingsOpen(false)}><X size={16} /></button>
          </div>
          <SettingsPanel />
        </div>
      </div>
    </div>
    </>
  );
}

/* ══════════════════════════════════════════════════════════════════════
   SUB-COMPONENTS
   ══════════════════════════════════════════════════════════════════════ */

function NavItem({ icon: Icon, label, meta, active = false, onClick }:
  { icon: LucideIcon; label: string; meta?: string; active?: boolean; onClick?: () => void }) {
  return (
    <button className={`n-item${active ? ' active' : ''}`} onClick={onClick}>
      <span className="n-ic"><Icon size={16} /></span>
      <span className="n-lbl">{label}</span>
      {meta && <span className="n-meta">{meta}</span>}
    </button>
  );
}

function ThemeBtn({ value, current, onClick }: { value: ThemeMode; current: ThemeMode; onClick: () => void }) {
  return <button className={current === value ? 'active' : ''} onClick={onClick}>{value}</button>;
}

function ToolBtn({ icon: Icon }: { icon: LucideIcon }) {
  return <button className="t-btn"><Icon size={13} /></button>;
}

function ModelSelector({
  models,
  value,
  onChange,
}: {
  models: any[];
  value: string;
  onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const pickerRef = useRef<HTMLDivElement>(null);
  
  const filteredModels = models.filter((model) => {
    const haystack = `${model.display_name} ${model.id}`.toLowerCase();
    return haystack.includes(query.toLowerCase());
  });
  
  const selectedModelData = models.find((model) => model.id === value);
  const selected = selectedModelData
      ? { label: selectedModelData.display_name || value, tier: selectedModelData.tier?.toUpperCase() || 'MED', provider: 'culi', modelName: selectedModelData.id }
      : { label: value, tier: 'MED', provider: 'culi', modelName: value };
  
  // Map tier to badge variant
  const getTierVariant = (tier: string) => {
    const t = tier?.toLowerCase();
    if (t === 'auto') return 'auto';
    if (t === 'fast') return 'fast';
    if (t === 'balanced') return 'balanced';
    if (t === 'powerful') return 'powerful';
    return 'balanced';
  };

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!pickerRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  const choose = (nextValue: string) => {
    onChange(nextValue);
    setOpen(false);
    setQuery('');
  };

  return (
    <div className={`model-picker${open ? ' open' : ''}`} ref={pickerRef}>
      <button
        type="button"
        className="model-picker-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`Selected model: ${selected.label}`}
        onClick={() => setOpen((current) => !current)}
      >
        <span className="model-picker-content">
          <ProviderLogo provider={selected.provider} modelName={selected.modelName} displayName={selected.label} size={15} wrapperClass="model-picker-logo" />
          <span className="model-picker-name">{selected.label}</span>
          <span className="model-picker-id">{selected.modelName}</span>
        </span>
        <ChevronDown className="model-picker-chevron" size={11} aria-hidden="true" />
      </button>
      {open && (
        <div className="model-menu" role="dialog" aria-label="Model picker">
          <label className="model-search">
            <Search size={12} aria-hidden="true" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search models..."
              autoFocus
              aria-label="Search models"
            />
          </label>
          <div className="model-menu-section model-menu-models">
            <div className="model-menu-heading"><span>MODELS</span><Plus size={12} aria-hidden="true" /></div>
            <div className="model-options" role="listbox" aria-label="Available models">
              {filteredModels.map((model) => (
                <button
                  type="button"
                  key={model.id}
                  className={`model-option${value === model.id ? ' selected' : ''}`}
                  role="option"
                  aria-selected={value === model.id}
                  onClick={() => choose(model.id)}
                >
                  <ProviderLogo provider="culi" modelName={model.id} displayName={model.display_name || model.id} size={16} wrapperClass="model-option-logo" />
                  <div className="model-option-info">
                    <span className="model-option-label">{model.display_name || model.id}</span>
                    <span className="model-option-id">{model.id}</span>
                  </div>
                  <span className="model-tier">{model.tier?.toUpperCase() || 'MED'}</span>
                  <ChevronRight size={11} />
                </button>
              ))}
              {!filteredModels.length && <div className="model-empty">No models found</div>}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function VizTab({ active, onClick, label }: { active: boolean; onClick: () => void; label: string }) {
  return <button className={`vt${active ? ' active' : ''}`} onClick={onClick}>{label}</button>;
}

/* ── Panel Components ─────────────────────────────────────────────── */
/* DagTraceTab, CodeReviewTab, KnowledgeGraphTab, KanbanTab
   are imported from ./components/Visualizer/* above              */

function EditorPanel() {
  return (
    <div className="ep">
      <div className="ep-tb"><Code2 size={13} /> No file opened</div>
      <div className="ep-empty"><Code2 size={36} /><p>Select a file to view</p></div>
    </div>
  );
}

function PreviewPanel() {
  return (
    <div className="pp">
      <div className="pp-bar"><span /><span /><span /><span className="pp-url">http://localhost:3000</span></div>
      <div className="pp-wait"><Globe size={36} /><p>Waiting for deployment…</p></div>
    </div>
  );
}



/* ── Token Saver Badge ──────────────────────────────────────────── */

function TokenSaverBadge() {
  // Simulated RTK metrics
  const stats = {
    savedTokens: 2847,
    totalTokens: 12450,
    savingsPct: 22.9,
    recentSessions: 14,
  };
  const barW = stats.savingsPct > 100 ? 100 : stats.savingsPct;
  return (
    <div className="tsb">
      <div className="tsb-hdr">
        <span className="tsb-icon">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <rect x="1" y="5" width="3" height="8" rx="0.5" fill="currentColor" opacity="0.6"/>
            <rect x="5" y="3" width="3" height="10" rx="0.5" fill="currentColor" opacity="0.8"/>
            <rect x="9" y="1" width="3" height="12" rx="0.5" fill="currentColor"/>
          </svg>
        </span>
        <div className="tsb-info">
          <strong>Token Saver</strong>
          <span className="tsb-pct">{stats.savingsPct}% saved</span>
        </div>
      </div>
      <div className="tsb-bar">
        <div className="tsb-bar-track">
          <div className="tsb-bar-fill" style={{ width: `${barW}%` }} />
        </div>
        <span className="tsb-bar-label">{stats.savedTokens.toLocaleString()} / {stats.totalTokens.toLocaleString()}</span>
      </div>
      <div className="tsb-meta">
        <span className="tsb-meta-item">
          <svg width="8" height="8" viewBox="0 0 8 8" fill="none"><circle cx="4" cy="4" r="3" fill="currentColor" opacity="0.5"/></svg>
          {stats.recentSessions} sessions
        </span>
        <span className="tsb-meta-item">
          <svg width="8" height="8" viewBox="0 0 8 8" fill="none"><circle cx="4" cy="4" r="3" fill="currentColor"/></svg>
          RTK active
        </span>
      </div>
    </div>
  );
}
/* ── Settings ──────────────────────────────────────────────────────── */

function SettingsPanel() {
  const [tab, setTab] = useState('general');
  const tabs = [
    { id: 'general', label: 'General', icon: Server },
    { id: 'provider', label: 'AI Provider', icon: Bot },
    { id: 'memory', label: 'Memory', icon: BookOpen },
    { id: 'skills', label: 'Skills', icon: Code2 },
  ];

  return (
    <div className="m-b">
      <nav className="set-nav">
        {tabs.map((t) => (
          <button key={t.id} className={`set-nav-i${tab === t.id ? ' active' : ''}`} onClick={() => setTab(t.id)}>
            <t.icon size={15} /> {t.label}
          </button>
        ))}
      </nav>
      <div className="set-body">
        {tab === 'general' && (
          <section>
            <h4>Workspace</h4>
            <div className="sg">
              <label>Project Directory</label>
              <div className="sp"><input type="text" readOnly value="/path/to/project" /><button className="s-btn">Browse</button></div>
            </div>
            <div className="sr">
              <div><div className="sl">Auto-Save</div><div className="sd">Automatically save edits</div></div>
              <label className="tgl"><input type="checkbox" defaultChecked /><span className="tgl-track" /></label>
            </div>
            <div className="sr" style={{ marginTop: 8 }}>
              <div><div className="sl">Token Saver</div><div className="sd">Optimize token usage via RTK</div></div>
              <label className="tgl"><input type="checkbox" /><span className="tgl-track" /></label>
            </div>
            <div className="sg">
              <label>RTK Threshold</label>
              <select defaultValue="0.5">
                <option value="0.3">Aggressive (0.3)</option>
                <option value="0.5">Balanced (0.5)</option>
                <option value="0.7">Conservative (0.7)</option>
              </select>
            </div>
          </section>
        )}
        {tab === 'provider' && (
          <section className="provider-settings">
            <h4>Fallback Tier Matrix</h4>
            <FallbackTierRow tier={1} label="Primary" provider="Anthropic" model="claude-3-5-sonnet" status="active" />
            <FallbackTierRow tier={2} label="Secondary" provider="OpenAI" model="gpt-4o" status="standby" />
            <FallbackTierRow tier={3} label="Emergency" provider="Groq" model="mixtral-8x7b" status="offline" />

            <div className="ps-section">
              <h4>Subagent → Model Mapping</h4>
              <SubagentModelMatrix />
            </div>

            <div className="ps-section">
              <h4>Multi-Account Key Pool</h4>
              <div className="key-pool">
                <KeyPoolRow provider="Anthropic" keyCount={3} />
                <KeyPoolRow provider="OpenAI" keyCount={2} />
                <KeyPoolRow provider="Groq" keyCount={1} />
              </div>
              <div className="sa" style={{ marginTop: 8 }}><button className="s-btn primary">Rotate API Keys</button></div>
            </div>

            <div className="ps-section">
              <h4>Router Endpoint</h4>
              <div className="sg">
                <label>Gateway URL</label>
                <input type="text" placeholder="https://gateway.culi.ai/v1/chat" />
              </div>
              <div className="sr">
                <div><div className="sl">Auto-Failover</div><div className="sd">Switch tier on quota exceeded</div></div>
                <label className="tgl"><input type="checkbox" defaultChecked /><span className="tgl-track" /></label>
              </div>
            </div>
          </section>
        )}
        {tab === 'memory' && <MemoryInspector />}
        {tab === 'skills' && <SkillsHub />}
      </div>
    </div>
  );
}

/* ── Provider Settings Sub-Components ────────────────────────── */

function FallbackTierRow({ tier, label, provider, model, status }:
  { tier: number; label: string; provider: string; model: string; status: 'active' | 'standby' | 'offline' }) {
  const statusColors = { active: '#28c840', standby: '#febc2e', offline: '#ff5f57' };
  return (
    <div className="tier-card">
      <div className="tier-hdr">
        <span className={`tier-badge tier-${tier}`}>TIER {tier}</span>
        <span className="tier-label">{label}</span>
        <span className="tier-status" style={{ color: statusColors[status] }}>
          <span className="tier-dot" style={{ background: statusColors[status] }} />
          {status}
        </span>
        <div className="tier-metrics">
          <span title="Response time"><Clock size={9} /> 1.2s</span>
          <span title="Token limit"><Hash size={9} /> 200K</span>
          <span title="Cost per request"><Server size={9} /> $0.003</span>
        </div>
      </div>
      <div className="tier-body">
        <div className="tier-row"><span>Provider</span><code>{provider}</code></div>
        <div className="tier-row"><span>Model</span><code>{model}</code></div>
        <div className="tier-row"><span>Usage</span><div className="tier-usage-track"><div className="tier-usage-fill" style={{ width: '68%' }} /></div></div>
      </div>
    </div>
  );
}

function SubagentModelMatrix() {
  const agents = [
    { name: 'Senior Architect', icon: Code2, model: 'claude-3-5-sonnet', temp: 0.2 },
    { name: 'Backend Dev', icon: Terminal, model: 'gpt-4o', temp: 0.4 },
    { name: 'Security Auditor', icon: Shield, model: 'claude-3-haiku', temp: 0.1 },
    { name: 'Tester', icon: Check, model: 'gpt-4o-mini', temp: 0.3 },
    { name: 'Designer', icon: Wand2, model: 'claude-3-5-sonnet', temp: 0.7 },
  ];

  return (
    <table className="subagent-matrix-table">
      <thead>
        <tr>
          <th>Agent</th>
          <th>Model</th>
          <th>Temp</th>
        </tr>
      </thead>
      <tbody>
        {agents.map((a) => (
          <tr key={a.name}>
            <td style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span className="mtd-ic"><a.icon size={13} /></span>
              <span>{a.name}</span>
            </td>
            <td><select defaultValue={a.model} className="mtd-sel">
              <option>claude-3-5-sonnet</option>
              <option>claude-3-haiku</option>
              <option>gpt-4o</option>
              <option>gpt-4o-mini</option>
              <option>mixtral-8x7b</option>
            </select></td>
            <td><div className="mtd-temp"><input type="range" min="0" max="1" step="0.1" defaultValue={a.temp} /><code>{a.temp}</code></div></td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function KeyPoolRow({ provider, keyCount }: { provider: string; keyCount: number }) {
  return (
    <div className="kp-row">
      <span className="kp-provider">{provider}</span>
      <div className="kp-keys">
        {Array.from({ length: keyCount }, (_, i) => (
          <span key={i} className="kp-key" title={`Key ${i + 1}`}>
            <span className="kp-key-dot" />
          </span>
        ))}
        <button className="kp-add" title="Add API key">+</button>
      </div>
      <span className="kp-count">{keyCount} keys</span>
    </div>
  );
}

/* ── Memory Tab Sub-Components ──────────────────────────────── */

const MEMORY_TIERS = [
  { id: 'working', label: 'Working', time: 'Active', items: 3, desc: 'Current session context and conversation history.' },
  { id: 'episodic', label: 'Episodic', time: '24h', items: 12, desc: 'Recent interactions and task outcomes.' },
  { id: 'semantic', label: 'Semantic', time: '7d', items: 89, desc: 'Learned patterns, concepts, and project knowledge.' },
  { id: 'procedural', label: 'Procedural', time: '30d', items: 45, desc: 'Reusable workflows, tool configurations, and scripts.' },
];

function MemoryInspector() {
  const [activeTier, setActiveTier] = useState('working');
  const current = MEMORY_TIERS.find(t => t.id === activeTier)!;
  return (
    <div className="mi">
      <div className="mi-tabs">
        {MEMORY_TIERS.map(t => (
          <button
            key={t.id}
            className={`mi-tab${activeTier === t.id ? ' active' : ''}`}
            onClick={() => setActiveTier(t.id)}
          >
            <span className="mi-tab-label">{t.label}</span>
            <span className="mi-tab-meta">{t.time}</span>
            <span className="mi-tab-count">{t.items}</span>
          </button>
        ))}
      </div>
      <div className="mi-body">
        {MEMORY_TIERS.filter(t => t.id === activeTier).map(t => (
          <MemoryTierCard key={t.id} tier={t} />
        ))}
        <div className="mi-actions">
          <button className="s-btn" style={{ fontSize: 8 }}>Clear All</button>
          <button className="s-btn" style={{ fontSize: 8 }}>Export</button>
        </div>
      </div>
    </div>
  );
}

function MemoryTierCard({ tier }: { tier: typeof MEMORY_TIERS[0] }) {
  return (
    <div className="mtc">
      <div className="mtc-hdr">
        <strong className="mtc-label">{tier.label} Memory</strong>
        <span className="mtc-time">{tier.time}</span>
      </div>
      <p className="mtc-desc">{tier.desc}</p>
      <div className="mtc-items">
        <div className="mtc-item"><span className="mtc-ic" /><span>Session state restored</span><code>2m ago</code></div>
        <div className="mtc-item"><span className="mtc-ic" /><span>User preferences loaded</span><code>15m ago</code></div>
        <div className="mtc-item"><span className="mtc-ic" /><span>Project context indexed</span><code>1h ago</code></div>
      </div>
    </div>
  );
}

/* ── Skills Hub Enhancement ──────────────────────────────────── */

const ALL_SKILLS = [
  { id: 'react-hooks', name: 'React Hooks', desc: 'React hooks best practices', enabled: true },
  { id: 'rust-errors', name: 'Rust Errors', desc: 'Rust error handling patterns', enabled: true },
  { id: 'ts-types', name: 'TS Types', desc: 'Advanced TypeScript generics', enabled: true },
  { id: 'api-design', name: 'API Design', desc: 'RESTful API patterns', enabled: false },
  { id: 'css-grid', name: 'CSS Grid', desc: 'Modern CSS layout techniques', enabled: true },
  { id: 'auth-flows', name: 'Auth Flows', desc: 'OAuth, JWT, session management', enabled: false },
  { id: 'perf-opt', name: 'Perf Optimization', desc: 'React/Vite performance tuning', enabled: true },
  { id: 'test-patterns', name: 'Test Patterns', desc: 'Vitest/Jest testing strategies', enabled: false },
];

function SkillsHub() {
  const [search, setSearch] = useState('');
  const [skills, setSkills] = useState(ALL_SKILLS);
  const filtered = skills.filter(s =>
    s.name.toLowerCase().includes(search.toLowerCase()) ||
    s.desc.toLowerCase().includes(search.toLowerCase())
  );
  const toggleSkill = (id: string) => {
    setSkills(prev => prev.map(s => s.id === id ? { ...s, enabled: !s.enabled } : s));
  };
  return (
    <div className="sh">
      <div className="sh-search">
        <Search size={11} />
        <input type="text" placeholder="Search skills…" value={search} onChange={e => setSearch(e.target.value)} />
      </div>
      <div className="sh-stats">
        <span>{skills.filter(s => s.enabled).length} enabled</span>
        <span>{skills.length} total</span>
      </div>
      <div className="sh-list">
        {filtered.map(s => (
          <div key={s.id} className="sh-card">
            <div className="sh-card-info">
              <strong>{s.name}</strong>
              <span>{s.desc}</span>
            </div>
            <label className="tgl-sm">
              <input type="checkbox" checked={s.enabled} onChange={() => toggleSkill(s.id)} />
              <span className="tgl-sm-track" />
            </label>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ── Composer Popups (Slash Menu & Context Mentions) ────────── */

function SlashMenu({ onSelect }: { onSelect: (cmd: string) => void }) {
  const [idx, setIdx] = useState(0);
  const cmds = [
    { id: '/plan', desc: 'Generate implementation plan', icon: Code2 },
    { id: '/test', desc: 'Write test suite', icon: Check },
    { id: '/review', desc: 'Code review analysis', icon: Search },
    { id: '/deploy', desc: 'Deploy to target', icon: Globe },
    { id: '/explain', desc: 'Explain code section', icon: BookOpen },
    { id: '/debug', desc: 'Debug and fix issues', icon: Terminal },
  ];
  return (
    <div className="slash-popup">
      {cmds.map((c, i) => (
        <div key={c.id} className={`slash-item${i === idx ? ' selected' : ''}`}
          onClick={() => onSelect(c.id)}
          onMouseEnter={() => setIdx(i)}
        >
          <c.icon size={13} />
          <span className="slash-cmd">{c.id}</span>
          <span className="slash-desc">{c.desc}</span>
        </div>
      ))}
    </div>
  );
}

function ContextMention({ type, onSelect }: { type: '@' | '#'; onSelect: (val: string) => void }) {
  const items = type === '@'
    ? [
        { id: '@Senior Architect', desc: 'Architecture planning', icon: Code2 },
        { id: '@Backend Dev', desc: 'API implementation', icon: Terminal },
        { id: '@Security', desc: 'Security audit', icon: Shield },
        { id: '@Tester', desc: 'Test automation', icon: Check },
      ]
    : [
        { id: '#Design_plan.md', desc: 'Project design doc', icon: FileText },
        { id: '#CULI/frontend', desc: 'Frontend source', icon: FolderOpen },
        { id: '#CULI/src', desc: 'Backend source', icon: FileCode2 },
        { id: '#CULI/skills', desc: 'Skills directory', icon: Code2 },
      ];
  return (
    <div className="slash-popup">
      {items.map(item => (
        <div key={item.id} className="slash-item" onClick={() => onSelect(item.id)}>
          <item.icon size={13} />
          <span className="slash-cmd">{item.id}</span>
          <span className="slash-desc">{item.desc}</span>
        </div>
      ))}
    </div>
  );
}

export default App;

/* ── Sidebar panel views CSS (injected globally via style tag) ──────── */
const SidebarPanelStyles = `
  .sv-panel { display:flex; flex-direction:column; height:100%; overflow:hidden; }
  .sv-header {
    display:flex; align-items:center; gap:6px;
    padding:10px 14px; font-size:11px; font-weight:700;
    text-transform:uppercase; letter-spacing:.06em;
    color:var(--color-ink-2); border-bottom:1px solid var(--color-rule);
    flex-shrink:0;
  }
  .sv-list { flex:1; overflow-y:auto; padding:6px 0; }
  .sv-item {
    display:flex; align-items:center; gap:8px;
    padding:8px 14px; cursor:pointer;
    transition:background .15s;
  }
  .sv-item:hover { background:var(--color-surface); }
  .sv-item-active { background:var(--color-surface); }
  .sv-item-info { display:flex; flex-direction:column; gap:2px; }
  .sv-item-title { font-size:12px; color:var(--color-ink); font-weight:500; }
  .sv-item-meta  { font-size:10px; color:var(--color-muted); font-family:monospace; }

  .sv-graph-nodes { display:flex; flex-direction:column; gap:8px; padding:12px; flex:1; overflow-y:auto; }
  .sv-node {
    display:flex; flex-direction:column; gap:3px;
    padding:10px 12px; border-radius:var(--radius-md);
    border:1px solid var(--color-rule);
    background:var(--color-surface);
  }
  .sv-node strong { font-size:12px; }
  .sv-node span   { font-size:10px; color:var(--color-muted); font-family:monospace; }
`;

// Inject styles once
if (typeof document !== 'undefined') {
  const styleId = 'culi-sv-styles';
  if (!document.getElementById(styleId)) {
    const s = document.createElement('style');
    s.id = styleId;
    s.textContent = SidebarPanelStyles;
    document.head.appendChild(s);
  }
}
