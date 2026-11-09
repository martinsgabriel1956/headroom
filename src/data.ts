export type Artifact = {
  id: string
  kind: string // folder name the rule matched
  rel: string // path relative to the project
  rule: string // the evidence that made it safe to remove
  real: number // bytes actually freed (block usage, hardlinks deduped)
  apparent: number // bytes as reported by file sizes
  days: number // days since last modification
}

export type Project = { id: string; name: string; path: string; days: number; artifacts: Artifact[] }

const GB = 1e9
const MB = 1e6

// Sample data until the Rust scanner is wired in. Sizes are invented.
export const SAMPLE: Project[] = [
  { id: 'editor', name: 'editor', path: '~/Documentos/development/personal/editor', days: 23, artifacts: [
    { id: 'editor/target', kind: 'target', rel: 'target', rule: 'Cargo.toml ao lado', real: 6.8 * GB, apparent: 6.9 * GB, days: 23 },
    { id: 'editor/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 512 * MB, apparent: 540 * MB, days: 23 },
  ] },
  { id: 'switch-terminal', name: 'switch-terminal', path: '~/Documentos/development/personal/switch-terminal', days: 2, artifacts: [
    { id: 'switch-terminal/target', kind: 'target', rel: 'src-tauri/target', rule: 'Cargo.toml ao lado', real: 4.1 * GB, apparent: 4.2 * GB, days: 2 },
    { id: 'switch-terminal/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 412 * MB, apparent: 430 * MB, days: 2 },
  ] },
  { id: 'sworn', name: 'sworn', path: '~/Documentos/development/personal/sworn', days: 41, artifacts: [
    { id: 'sworn/target', kind: 'target', rel: 'target', rule: 'Cargo.toml ao lado', real: 2.3 * GB, apparent: 2.3 * GB, days: 41 },
  ] },
  { id: 'dev-brain', name: 'dev-brain', path: '~/Documentos/dev-brain', days: 60, artifacts: [
    { id: 'dev-brain/.venv', kind: '.venv', rel: '.venv', rule: 'pyvenv.cfg dentro', real: 1.1 * GB, apparent: 1.1 * GB, days: 60 },
    { id: 'dev-brain/__pycache__', kind: '__pycache__', rel: 'app/__pycache__', rule: 'sempre cache', real: 14 * MB, apparent: 14 * MB, days: 60 },
  ] },
  { id: 'vilazen', name: 'vilazen', path: '~/Documentos/development/personal/vilazen', days: 5, artifacts: [
    { id: 'vilazen/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 540 * MB, apparent: 560 * MB, days: 5 },
    { id: 'vilazen/.next', kind: '.next', rel: '.next', rule: 'package.json ao lado', real: 310 * MB, apparent: 310 * MB, days: 5 },
  ] },
  { id: 'http-client', name: 'http-client', path: '~/Documentos/development/personal/http-client', days: 15, artifacts: [
    { id: 'http-client/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 820 * MB, apparent: 850 * MB, days: 15 },
  ] },
  { id: 'docker-gui', name: 'docker-gui', path: '~/Documentos/development/docker-gui', days: 30, artifacts: [
    { id: 'docker-gui/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 380 * MB, apparent: 395 * MB, days: 30 },
    { id: 'docker-gui/__pycache__', kind: '__pycache__', rel: '__pycache__', rule: 'sempre cache', real: 2 * MB, apparent: 2 * MB, days: 30 },
  ] },
  { id: 'db-manager', name: 'db-manager', path: '~/Documentos/development/personal/db-manager', days: 9, artifacts: [
    { id: 'db-manager/node_modules', kind: 'node_modules', rel: 'node_modules', rule: 'package.json ao lado', real: 84 * MB, apparent: 690 * MB, days: 9 },
  ] },
]

export const ACTIVE_DAYS = 7
