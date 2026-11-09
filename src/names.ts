import type { Project } from './data'

/** Copies for rendering. Projects sharing a folder name (git worktrees such as `main`, `develop`)
 *  take as many parent folders as it takes to tell them apart: `web/main`, `api/feat/controle-pedra`. */
export function withUniqueNames(projects: Project[]): Project[] {
  const parts = projects.map((p) => p.id.split('/').filter(Boolean))
  const depth = parts.map(() => 1)
  const label = (i: number) => (parts[i].length ? parts[i].slice(-depth[i]).join('/') : projects[i].name)
  for (let changed = true; changed; ) {
    changed = false
    const seen = new Map<string, number[]>()
    projects.forEach((_, i) => seen.set(label(i), [...(seen.get(label(i)) ?? []), i]))
    for (const group of seen.values()) {
      if (group.length < 2) continue
      for (const i of group) {
        if (depth[i] < parts[i].length) {
          depth[i]++
          changed = true
        }
      }
    }
  }
  return projects.map((p, i) => ({ ...p, name: depth[i] > 1 ? label(i) : p.name, artifacts: [...p.artifacts] }))
}
