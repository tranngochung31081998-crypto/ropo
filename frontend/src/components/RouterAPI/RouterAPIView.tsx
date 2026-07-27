import { useEffect } from 'react';
import type React from 'react';
import { useRouterStore, RouterTab } from './store';
import DashboardPanel from './DashboardPanel';
import FreeModelPanel from './FreeModelPanel';
import QverisPanel from './QverisPanel';
import CustomProvidersPanel from './CustomProvidersPanel';
import ModelCatalogPanel from './ModelCatalogPanel';
import PlaygroundPanel from './PlaygroundPanel';
import EngineerModelPanel from './EngineerModelPanel';
import {
  LayoutDashboard, Zap, Shield, Key, Box, TerminalSquare, Cpu,
} from 'lucide-react';

const TABS: { id: RouterTab; label: string; hint: string; Icon: React.ComponentType<{ className?: string }> }[] = [
  { id: 'dashboard',  label: 'Dashboard',   hint: 'Realtime stats',        Icon: LayoutDashboard },
  { id: 'freemodel',  label: 'CULI Free',   hint: 'Unified Blackbox+Sixth', Icon: Zap },
  { id: 'qveris',     label: 'Qveris',      hint: 'Key pool + credits',    Icon: Shield },
  { id: 'engineer',   label: 'Engineers',   hint: 'Model per role',        Icon: Cpu },
  { id: 'custom',     label: 'Providers',   hint: 'OpenAI / Claude / etc',  Icon: Key },
  { id: 'models',     label: 'Models',      hint: '30+ model catalog',      Icon: Box },
  { id: 'playground', label: 'Playground',  hint: 'Stream test',            Icon: TerminalSquare },
];

export default function RouterAPIView() {
  const activeTab    = useRouterStore(s => s.activeTab);
  const setActiveTab = useRouterStore(s => s.setActiveTab);
  const syncFromRouter = useRouterStore(s => s.syncFromRouter);

  // Sync real stats from CulirouterAPI when Router tab opens
  useEffect(() => {
    syncFromRouter();
  }, []);

  return (
    <div className="ra-root">
      <div className="ra-head">
        <div className="ra-head-left">
          <div className="ra-emblem" aria-hidden>
            <svg viewBox="0 0 40 40" className="ra-emblem-svg">
              <defs>
                <linearGradient id="ra-grad" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="1"/>
                  <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0.5"/>
                </linearGradient>
              </defs>
              <path d="M20 3 L34 10 V25 C34 33 27 38 20 38 C13 38 6 33 6 25 V10 Z"
                    fill="none" stroke="url(#ra-grad)" strokeWidth="2.2"/>
              <path d="M12 20 L18 26 L29 14" fill="none" stroke="var(--color-accent)"
                    strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
          <div>
            <h2 className="ra-title">CULI · Router API</h2>
            <p className="ra-sub">Unified OpenAI-compatible gateway · endpoint <code className="ra-mono">/v1/chat/completions</code></p>
          </div>
        </div>
        <div className="ra-head-right">
          <span className="ra-endpoint-pill">
            <span className="ra-dot ra-dot-ok"/>
            <span>Port 4000 · SSE streaming</span>
          </span>
        </div>
      </div>

      <nav className="ra-tabs" role="tablist">
        {TABS.map(t => {
          const active = activeTab === t.id;
          return (
            <button
              key={t.id}
              role="tab"
              aria-selected={active}
              className={`ra-tab ${active ? 'ra-tab-active' : ''}`}
              onClick={() => setActiveTab(t.id)}
            >
              <t.Icon className="ra-tab-ic"/>
              <span className="ra-tab-main">
                <span className="ra-tab-label">{t.label}</span>
                <span className="ra-tab-hint">{t.hint}</span>
              </span>
              {active && <span className="ra-tab-bar"/>}
            </button>
          );
        })}
      </nav>

      <section className="ra-body" key={activeTab} aria-live="polite">
        {activeTab === 'dashboard' && <DashboardPanel />}
        {activeTab === 'freemodel' && <FreeModelPanel />}
        {activeTab === 'qveris' && <QverisPanel />}
        {activeTab === 'engineer' && <EngineerModelPanel />}
        {activeTab === 'custom' && <CustomProvidersPanel />}
        {activeTab === 'models' && <ModelCatalogPanel />}
        {activeTab === 'playground' && <PlaygroundPanel />}
      </section>
    </div>
  );
}
