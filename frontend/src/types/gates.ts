// Gate data types matching backend Rust structures

export type GateCategory = 'Security' | 'Performance' | 'CodeQuality' | 'Architecture';

export type Severity = 'Critical' | 'High' | 'Medium' | 'Low';

export interface GateViolation {
  gate_id: number;
  gate_name: string;
  severity: Severity;
  file: string;
  line: number;
  column?: number;
  message: string;
  snippet: string;
  suggested_fix?: string;
  auto_fixable: boolean;
}

export interface GateStats {
  total_files: number;
  total_violations: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  by_category: Record<string, number>;
}

export interface GateReport {
  violations: GateViolation[];
  stats: GateStats;
  scanned_files: string[];
  scan_duration_ms: number;
  timestamp: string;
}

export interface GateDefinition {
  id: number;
  name: string;
  category: GateCategory;
  severity: Severity;
  description: string;
}

// Gate catalog - 15 gates from backend
export const GATE_CATALOG: GateDefinition[] = [
  // Security Gates (5)
  { id: 1, name: 'Hardcoded Secrets Detection', category: 'Security', severity: 'Critical', description: 'Detects hardcoded API keys, passwords, tokens' },
  { id: 2, name: 'SQL Injection Prevention', category: 'Security', severity: 'Critical', description: 'Detects SQL injection through string concatenation' },
  { id: 3, name: 'Authentication Bypass Detection', category: 'Security', severity: 'Critical', description: 'Detects commented auth checks or always-true conditions' },
  { id: 4, name: 'Unsafe Deserialization', category: 'Security', severity: 'High', description: 'Detects deserialization without validation' },
  { id: 5, name: 'Path Traversal Prevention', category: 'Security', severity: 'High', description: 'Detects file operations with user-controlled paths' },
  
  // Performance Gates (3)
  { id: 6, name: 'N+1 Query Detection', category: 'Performance', severity: 'High', description: 'Detects database queries inside loops' },
  { id: 7, name: 'Unbounded Resource Allocation', category: 'Performance', severity: 'High', description: 'Detects memory allocation without size limits' },
  { id: 8, name: 'Blocking in Async Context', category: 'Performance', severity: 'Medium', description: 'Detects blocking I/O in async functions' },
  
  // Code Quality Gates (4)
  { id: 9, name: 'Error Swallowing Detection', category: 'CodeQuality', severity: 'Medium', description: 'Detects silently ignored errors' },
  { id: 10, name: 'Magic Numbers', category: 'CodeQuality', severity: 'Low', description: 'Detects unexplained numeric literals' },
  { id: 11, name: 'TODO in Production', category: 'CodeQuality', severity: 'Medium', description: 'Detects unresolved TODO/FIXME comments' },
  { id: 12, name: 'Missing Error Context', category: 'CodeQuality', severity: 'Low', description: 'Detects error propagation without context' },
  
  // Architecture Gates (3)
  { id: 13, name: 'Unnecessary Dependencies', category: 'Architecture', severity: 'Low', description: 'Detects heavyweight crates for simple tasks' },
  { id: 14, name: 'God Object/Module', category: 'Architecture', severity: 'Medium', description: 'Detects modules with too many responsibilities' },
  { id: 15, name: 'Untestable Code', category: 'Architecture', severity: 'Medium', description: 'Detects tight coupling to concrete types' },
];
