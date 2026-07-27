import React, { useState, useEffect } from 'react';
import { AlertTriangle, CheckCircle2, ShieldAlert, Sparkles, XCircle, FileCode2, Shield, Zap, Code2, Layers } from 'lucide-react';
import { GATE_CATALOG, type GateReport, type GateViolation, type Severity } from '../../types/gates';

export function CodeReviewTab() {
  const [activeTab, setActiveTab] = useState<'gates' | 'vulnerabilities' | 'stats'>('gates');
  const [gateReport, setGateReport] = useState<GateReport | null>(null);
  const [loading, setLoading] = useState(false);

  // Mock data until backend integration
  const mockReport: GateReport = {
    violations: [
      {
        gate_id: 1,
        gate_name: 'Hardcoded Secrets Detection',
        severity: 'Critical',
        file: 'src/config/mod.rs',
        line: 15,
        message: 'Hardcoded OpenAI API key detected',
        snippet: 'let api_key = "sk-proj-abc123...";',
        suggested_fix: 'Use env::var("OPENAI_API_KEY") or config file instead',
        auto_fixable: false,
      },
      {
        gate_id: 6,
        gate_name: 'N+1 Query Detection',
        severity: 'High',
        file: 'src/database/users.rs',
        line: 78,
        message: 'N+1 query detected: database call inside loop',
        snippet: 'for user_id in user_ids { let user = query!(...)',
        suggested_fix: 'Use batch query with WHERE id = ANY($1) or JOIN',
        auto_fixable: false,
      },
      {
        gate_id: 9,
        gate_name: 'Error Swallowing Detection',
        severity: 'Medium',
        file: 'src/tools/terminal.rs',
        line: 42,
        message: 'Error swallowed with unwrap()',
        snippet: 'let result = api_call().unwrap();',
        suggested_fix: 'Use ? operator or proper error handling',
        auto_fixable: true,
      },
    ],
    stats: {
      total_files: 47,
      total_violations: 3,
      critical: 1,
      high: 1,
      medium: 1,
      low: 0,
      by_category: {
        'Security': 1,
        'Performance': 1,
        'CodeQuality': 1,
      },
    },
    scanned_files: [],
    scan_duration_ms: 1245,
    timestamp: new Date().toISOString(),
  };

  useEffect(() => {
    // Simulate loading gate report
    setGateReport(mockReport);
  }, []);

  const getSeverityColor = (severity: Severity) => {
    switch (severity) {
      case 'Critical': return 'var(--color-error)';
      case 'High': return 'var(--color-warning)';
      case 'Medium': return 'var(--color-caution)';
      case 'Low': return 'var(--color-muted)';
    }
  };

  const getSeverityIcon = (severity: Severity) => {
    switch (severity) {
      case 'Critical': return <ShieldAlert size={12} />;
      case 'High': return <AlertTriangle size={12} />;
      case 'Medium': return <AlertTriangle size={12} />;
      case 'Low': return <AlertTriangle size={12} />;
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'Security': return <Shield size={12} />;
      case 'Performance': return <Zap size={12} />;
      case 'CodeQuality': return <Code2 size={12} />;
      case 'Architecture': return <Layers size={12} />;
      default: return <Code2 size={12} />;
    }
  };

  const qualityScore = gateReport 
    ? ((gateReport.stats.total_files * 15 - gateReport.stats.total_violations) / (gateReport.stats.total_files * 15) * 100).toFixed(1)
    : '0.0';

  const passedGates = gateReport 
    ? GATE_CATALOG.filter(gate => !gateReport.violations.some(v => v.gate_id === gate.id))
    : [];

  const failedGates = gateReport
    ? GATE_CATALOG.filter(gate => gateReport.violations.some(v => v.gate_id === gate.id))
    : [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)', height: '100%' }}>
      {/* Quality Score Gauge */}
      <div className="quality-gauge">
        <div>
          <div style={{ fontSize: '11px', color: 'var(--color-muted)', fontWeight: 600, textTransform: 'uppercase' }}>
            Security Quality Index
          </div>
          <div style={{ fontSize: '12px', color: 'var(--color-ink-2)', marginTop: '2px' }}>
            {GATE_CATALOG.length} Security Gates • {gateReport?.stats.total_files || 0} Files Scanned
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <span className="quality-score-num">{qualityScore}%</span>
          <div style={{ fontSize: '10px', color: gateReport && gateReport.stats.critical === 0 ? 'var(--color-success)' : 'var(--color-error)', display: 'flex', alignItems: 'center', gap: '2px', justifyContent: 'flex-end' }}>
            {gateReport && gateReport.stats.critical === 0 ? (
              <><CheckCircle2 size={10} /> PASSED</>
            ) : (
              <><XCircle size={10} /> FAILED</>
            )}
          </div>
        </div>
      </div>

      {/* Stats Summary */}
      {gateReport && (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '4px' }}>
          <div className="stat-card" style={{ borderLeft: '3px solid var(--color-error)' }}>
            <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-error)' }}>{gateReport.stats.critical}</div>
            <div style={{ fontSize: '9px', color: 'var(--color-muted)' }}>Critical</div>
          </div>
          <div className="stat-card" style={{ borderLeft: '3px solid var(--color-warning)' }}>
            <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-warning)' }}>{gateReport.stats.high}</div>
            <div style={{ fontSize: '9px', color: 'var(--color-muted)' }}>High</div>
          </div>
          <div className="stat-card" style={{ borderLeft: '3px solid var(--color-caution)' }}>
            <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-caution)' }}>{gateReport.stats.medium}</div>
            <div style={{ fontSize: '9px', color: 'var(--color-muted)' }}>Medium</div>
          </div>
          <div className="stat-card" style={{ borderLeft: '3px solid var(--color-muted)' }}>
            <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-ink)' }}>{gateReport.stats.low}</div>
            <div style={{ fontSize: '9px', color: 'var(--color-muted)' }}>Low</div>
          </div>
        </div>
      )}

      {/* Internal View Switcher */}
      <div style={{ display: 'flex', gap: '4px', borderBottom: '1px solid var(--color-rule)', paddingBottom: '4px' }}>
        <button
          className={`mode-btn ${activeTab === 'gates' ? 'active' : ''}`}
          onClick={() => setActiveTab('gates')}
        >
          All Gates ({GATE_CATALOG.length})
        </button>
        <button
          className={`mode-btn ${activeTab === 'vulnerabilities' ? 'active' : ''}`}
          onClick={() => setActiveTab('vulnerabilities')}
        >
          Violations ({gateReport?.stats.total_violations || 0})
        </button>
        <button
          className={`mode-btn ${activeTab === 'stats' ? 'active' : ''}`}
          onClick={() => setActiveTab('stats')}
        >
          Statistics
        </button>
      </div>

      {/* Tab 1: All Gates */}
      {activeTab === 'gates' && (
        <div style={{ display: 'flex', flexDirection: 'column', overflowY: 'auto', gap: '2px' }}>
          {/* Failed Gates */}
          {failedGates.length > 0 && (
            <>
              <div style={{ fontSize: '10px', fontWeight: 600, color: 'var(--color-error)', padding: '4px 0', textTransform: 'uppercase' }}>
                Failed ({failedGates.length})
              </div>
              {failedGates.map((gate) => {
                const violations = gateReport?.violations.filter(v => v.gate_id === gate.id) || [];
                return (
                  <div key={gate.id} className="slop-gate-item" style={{ borderLeft: `3px solid ${getSeverityColor(gate.severity)}` }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1 }}>
                      <XCircle size={12} color="var(--color-error)" />
                      <div>
                        <div>{gate.name}</div>
                        <div style={{ fontSize: '9px', color: 'var(--color-muted)' }}>
                          {violations.length} violation{violations.length !== 1 ? 's' : ''}
                        </div>
                      </div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                      {getCategoryIcon(gate.category)}
                      <span className="badge-role" style={{ fontSize: '9px' }}>{gate.category}</span>
                    </div>
                  </div>
                );
              })}
            </>
          )}

          {/* Passed Gates */}
          {passedGates.length > 0 && (
            <>
              <div style={{ fontSize: '10px', fontWeight: 600, color: 'var(--color-success)', padding: '4px 0', textTransform: 'uppercase', marginTop: '8px' }}>
                Passed ({passedGates.length})
              </div>
              {passedGates.map((gate) => (
                <div key={gate.id} className="slop-gate-item" style={{ opacity: 0.7 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1 }}>
                    <CheckCircle2 size={12} color="var(--color-success)" />
                    <span>{gate.name}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                    {getCategoryIcon(gate.category)}
                    <span className="badge-role" style={{ fontSize: '9px' }}>{gate.category}</span>
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      )}

      {/* Tab 2: Violations Detail */}
      {activeTab === 'vulnerabilities' && gateReport && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', overflowY: 'auto' }}>
          {gateReport.violations.map((violation, idx) => (
            <div key={idx} className={`vulnerability-card ${violation.severity.toLowerCase()}`}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  {getSeverityIcon(violation.severity)}
                  <strong style={{ fontSize: '11px', color: 'var(--color-ink)' }}>
                    Gate #{violation.gate_id}: {violation.gate_name}
                  </strong>
                </div>
                <span 
                  className="badge-role" 
                  style={{ 
                    textTransform: 'uppercase', 
                    fontSize: '9px',
                    background: getSeverityColor(violation.severity),
                    color: 'white'
                  }}
                >
                  {violation.severity}
                </span>
              </div>

              <div style={{ fontSize: '10px', color: 'var(--color-muted)', marginBottom: '6px', fontFamily: 'var(--font-mono)' }}>
                <FileCode2 size={10} style={{ display: 'inline', marginRight: '4px' }} />
                {violation.file}:{violation.line}
              </div>

              <p style={{ fontSize: '11px', color: 'var(--color-neutral)', lineHeight: '1.4', marginBottom: '6px' }}>
                {violation.message}
              </p>

              {/* Code Snippet */}
              <pre style={{
                fontSize: '10px',
                fontFamily: 'var(--font-mono)',
                background: 'var(--color-paper-3)',
                padding: '6px 8px',
                borderRadius: '4px',
                marginBottom: '6px',
                overflowX: 'auto',
                color: 'var(--color-ink-2)'
              }}>
                {violation.snippet}
              </pre>

              {/* Suggested Fix */}
              {violation.suggested_fix && (
                <div style={{
                  fontSize: '10px',
                  padding: '6px 8px',
                  background: 'oklch(from var(--color-accent) l c h / 0.1)',
                  borderLeft: '3px solid var(--color-accent)',
                  borderRadius: '4px',
                  marginBottom: '6px'
                }}>
                  <strong>💡 Suggested Fix:</strong> {violation.suggested_fix}
                </div>
              )}

              {violation.auto_fixable && (
                <button className="mode-btn active" style={{ fontSize: '9px', width: 'fit-content' }}>
                  <Sparkles size={10} /> Auto-Fix Available
                </button>
              )}
            </div>
          ))}

          {gateReport.violations.length === 0 && (
            <div style={{ 
              textAlign: 'center', 
              padding: '32px', 
              color: 'var(--color-success)',
              fontSize: '12px' 
            }}>
              <CheckCircle2 size={48} style={{ margin: '0 auto 12px' }} />
              <div style={{ fontWeight: 600 }}>No Violations Found!</div>
              <div style={{ fontSize: '10px', color: 'var(--color-muted)', marginTop: '4px' }}>
                All {GATE_CATALOG.length} security gates passed
              </div>
            </div>
          )}
        </div>
      )}

      {/* Tab 3: Statistics */}
      {activeTab === 'stats' && gateReport && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', overflowY: 'auto' }}>
          {/* Scan Info */}
          <div className="stat-card">
            <div style={{ fontSize: '11px', fontWeight: 600, marginBottom: '8px' }}>Scan Information</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', fontSize: '10px' }}>
              <div>
                <div style={{ color: 'var(--color-muted)' }}>Files Scanned</div>
                <div style={{ fontWeight: 600, fontSize: '14px' }}>{gateReport.stats.total_files}</div>
              </div>
              <div>
                <div style={{ color: 'var(--color-muted)' }}>Duration</div>
                <div style={{ fontWeight: 600, fontSize: '14px' }}>{gateReport.scan_duration_ms}ms</div>
              </div>
              <div>
                <div style={{ color: 'var(--color-muted)' }}>Timestamp</div>
                <div style={{ fontWeight: 600, fontSize: '10px', fontFamily: 'var(--font-mono)' }}>
                  {new Date(gateReport.timestamp).toLocaleString()}
                </div>
              </div>
            </div>
          </div>

          {/* Category Breakdown */}
          <div className="stat-card">
            <div style={{ fontSize: '11px', fontWeight: 600, marginBottom: '8px' }}>Violations by Category</div>
            {Object.entries(gateReport.stats.by_category).map(([category, count]) => (
              <div key={category} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 0', fontSize: '10px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  {getCategoryIcon(category)}
                  <span>{category}</span>
                </div>
                <span style={{ fontWeight: 600 }}>{count}</span>
              </div>
            ))}
          </div>

          {/* Quality Score Breakdown */}
          <div className="stat-card">
            <div style={{ fontSize: '11px', fontWeight: 600, marginBottom: '8px' }}>Quality Metrics</div>
            <div style={{ fontSize: '10px', color: 'var(--color-muted)', marginBottom: '8px' }}>
              Score: ({gateReport.stats.total_files} files × 15 gates - {gateReport.stats.total_violations} violations) / ({gateReport.stats.total_files} × 15) × 100
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ flex: 1, height: '8px', background: 'var(--color-paper-3)', borderRadius: '4px', overflow: 'hidden' }}>
                <div style={{ 
                  height: '100%', 
                  width: `${qualityScore}%`, 
                  background: parseFloat(qualityScore) > 90 ? 'var(--color-success)' : parseFloat(qualityScore) > 70 ? 'var(--color-warning)' : 'var(--color-error)',
                  transition: 'width 0.3s ease'
                }} />
              </div>
              <span style={{ fontWeight: 700, fontSize: '14px' }}>{qualityScore}%</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
