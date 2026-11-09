# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack
Tauri 2 desktop app for Linux. Rust backend (scan, size, delete; own parallel traversal on `std::fs` + `rayon`). React 19 + Vite + Tailwind 4 frontend, plain `useState`. Same base as the user's db-manager.

## Users
A developer with many local projects under `~/Documentos/development` whose disk fills up with regenerable build artifacts. Runs it occasionally when space is low or as routine housekeeping.

## Product Purpose
Find and delete regenerable dev artifacts (`node_modules`, `target`, `.venv`, `__pycache__`, `.next`, `.turbo`, `.gradle`, caches, later orphan Docker images) across all projects, showing how much space is actually freed. Success: real gigabytes back, with zero source or config lost.

## Positioning
- Every match is validated by a rule (for example, `node_modules` only when a sibling `package.json` exists), not by folder name alone.
- The number shown is real disk freed: block usage with hardlink dedup by inode, so pnpm-linked `node_modules` is not overstated.
- Projects modified in the last 7 days are flagged as active.

## Operating Context
Linux desktop. Scan root picked with the native folder dialog. Results stream in during the scan via Tauri events. Delete takes ids from the last scan only, never raw paths from the webview, and re-validates each rule before removing.

## Capabilities and Constraints
- Deletion is permanent (`remove_dir_all`), not trash: on Linux the trash lives on the same disk and frees nothing.
- `dist/` and `build/` are off by default (sometimes versioned).
- Phases: v0 scan + list + real delete (see specs/prd-varredura-e-limpeza), v2 Docker tab, v3 packaging (.deb / AppImage).
- One scan root per session, default `~` (hidden folders included), not remembered between sessions; scans on open; nothing preselected.
- Undecided: config file for custom rules, scheduling.

## Brand Commitments
- Name: **Headroom** (binary/package: `headroom`). Meaning: spare room, a safety margin; it names the outcome (free disk), not the act of deleting. Chosen over kerf, tare, husk, coppice, decant, fettle, pare, ballast.
- Personality: trustworthy and precise. A serious tool that never removes what it shouldn't, and reports the true number.

## Evidence on Hand
None yet. No users, benchmarks or screenshots exist; do not fabricate them.

## Product Principles
1. Never delete what can't be regenerated: rules validate, the backend re-checks.
2. Report the true number, never the flattering one.
3. Show progress as it happens: results stream in during the scan instead of waiting behind a spinner.
4. One screen, one job.
