import { Channel, invoke, isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useCallback, useEffect, useRef, useState } from 'react'
import { SAMPLE, type Project } from './data'

// Wire format of the Rust commands (src-tauri/src/lib.rs).
type Dto = { id: number; project: string; kind: string; rel: string; rule: string; real: number; apparent: number; days: number }
type ScanEvent =
  | { type: 'started'; generation: number; root: string; home: string }
  | { type: 'artifact'; artifact: Dto }
  | { type: 'progress'; dirs: number }
  | { type: 'done'; millis: number; skipped: number; count: number; active: { project: string; days: number }[] }
type DeleteEvent = { type: 'deleting' | 'deleted'; id: number } | { type: 'refused'; id: number; reason: string }
type Report = { freed: number; removed: number[]; refused: { id: number; reason: string }[] }

export type Status =
  | { phase: 'scanning'; dirs: number }
  | { phase: 'done'; millis: number; skipped: number; count: number }
  | { phase: 'error'; message: string }
export type DeleteResult = { freed: number; refused: { label: string; reason: string }[] }

// Outside the app (plain `npm run dev` in a browser) the screen runs on sample data.
const live = isTauri()

export function useHeadroom() {
  // The ref owns its own copies: deletions edit them in place, never the rendered state or SAMPLE.
  const projects = useRef(new Map<string, Project>(live ? [] : SAMPLE.map((p) => [p.id, { ...p, artifacts: [...p.artifacts] }])))
  const [view, setView] = useState<Project[]>(live ? [] : SAMPLE)
  const [root, setRoot] = useState(live ? '' : '~/Documentos/development')
  const [status, setStatus] = useState<Status>(
    live ? { phase: 'scanning', dirs: 0 } : { phase: 'done', millis: 0, skipped: 0, count: SAMPLE.flatMap((p) => p.artifacts).length },
  )
  const generation = useRef(0)
  const home = useRef('')
  const absRoot = useRef<string | undefined>(undefined)
  const frame = useRef(0)

  const tilde = (p: string) => (home.current && p.startsWith(home.current) ? '~' + p.slice(home.current.length) : p)
  // Many artifacts arrive per frame; render at most once per frame.
  const flush = () => {
    cancelAnimationFrame(frame.current)
    frame.current = requestAnimationFrame(() => setView([...projects.current.values()].map((p) => ({ ...p, artifacts: [...p.artifacts] }))))
  }

  const scan = useCallback((dir?: string) => {
    if (!live) return
    projects.current = new Map()
    setView([])
    setStatus({ phase: 'scanning', dirs: 0 })
    let mine = -1
    const ch = new Channel<ScanEvent>()
    ch.onmessage = (e) => {
      if (e.type === 'started') {
        mine = generation.current = e.generation
        home.current = e.home
        absRoot.current = e.root
        setRoot(tilde(e.root))
        return
      }
      if (mine !== generation.current) return // a newer scan took over
      if (e.type === 'artifact') {
        const a = e.artifact
        let p = projects.current.get(a.project)
        if (!p) {
          p = { id: a.project, name: a.project.split('/').pop() || a.project, path: tilde(a.project), days: Infinity, artifacts: [] }
          projects.current.set(a.project, p)
        }
        p.artifacts.push({ id: String(a.id), kind: a.kind, rel: a.rel, rule: a.rule, real: a.real, apparent: a.apparent, days: a.days })
        flush()
      } else if (e.type === 'progress') {
        setStatus((s) => (s.phase === 'scanning' ? { phase: 'scanning', dirs: e.dirs } : s))
      } else {
        for (const x of e.active) {
          const p = projects.current.get(x.project)
          if (p) p.days = x.days
        }
        flush()
        setStatus({ phase: 'done', millis: e.millis, skipped: e.skipped, count: e.count })
      }
    }
    invoke('scan', { root: dir ?? null, onEvent: ch }).catch((err) => setStatus({ phase: 'error', message: String(err) }))
  }, [])

  useEffect(() => scan(), [scan])

  const all = () => [...projects.current.values()].flatMap((p) => p.artifacts.map((a) => ({ p, a })))
  // Stable identity: callers use it as an effect dependency. It only reads refs.
  const measure = useCallback(async (ids: string[]): Promise<number> => {
    if (live) return invoke<number>('measure', { generation: generation.current, ids: ids.map(Number) })
    const set = new Set(ids)
    return [...projects.current.values()].flatMap((p) => p.artifacts).filter((a) => set.has(a.id)).reduce((s, a) => s + a.real, 0)
  }, [])

  return {
    live,
    root,
    projects: view,
    status,
    /** Resolves true when a folder was picked and a new scan started. */
    async chooseRoot(): Promise<boolean> {
      if (!live) return false
      const dir = await open({ directory: true, defaultPath: absRoot.current })
      if (typeof dir !== 'string') return false
      scan(dir)
      return true
    },
    rescan: () => scan(absRoot.current),
    measure,
    async remove(ids: string[], onProgress: (done: number) => void): Promise<DeleteResult> {
      const labels = new Map(all().map(({ p, a }) => [a.id, `${p.name}/${a.rel}`]))
      let report: Report
      if (live) {
        let done = 0
        const ch = new Channel<DeleteEvent>()
        ch.onmessage = (e) => e.type !== 'deleting' && onProgress(++done)
        report = await invoke<Report>('delete', { generation: generation.current, ids: ids.map(Number), onEvent: ch })
      } else {
        // Sample mode only drops the items from the screen.
        report = { freed: await measure(ids), removed: ids.map(Number), refused: [] }
      }
      const gone = new Set(live ? report.removed.map(String) : ids)
      for (const [k, p] of projects.current) {
        const artifacts = p.artifacts.filter((a) => !gone.has(a.id))
        if (artifacts.length) projects.current.set(k, { ...p, artifacts })
        else projects.current.delete(k)
      }
      flush()
      return { freed: report.freed, refused: report.refused.map((r) => ({ label: labels.get(String(r.id)) ?? String(r.id), reason: r.reason })) }
    },
  }
}
