import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { ACTIVE_DAYS, type Artifact, type Project } from './data'
import { useHeadroom, type DeleteResult, type Status } from './scan'
import { squarify, type Rect } from './treemap'

type Mode = 'real' | 'apparent'

const fmt = (b: number) =>
  b >= 1e9
    ? `${(b / 1e9).toLocaleString('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 })} GB`
    : `${Math.max(1, Math.round(b / 1e6))} MB`
const ago = (d: number) => (d === 0 ? 'hoje' : d === 1 ? 'ontem' : `há ${d} dias`)
const size = (a: Artifact, m: Mode) => (m === 'real' ? a.real : a.apparent)
const total = (p: Project, m: Mode) => p.artifacts.reduce((s, a) => s + size(a, m), 0)
const isActive = (p: Project) => p.days < ACTIVE_DAYS

export default function App() {
  const h = useHeadroom()
  const projects = h.projects
  const [mode, setMode] = useState<Mode>('real')
  const [selected, setSelected] = useState<Set<string>>(() => new Set())
  const [hover, setHover] = useState<string | null>(null)
  const [result, setResult] = useState<DeleteResult | null>(null)
  const [deleting, setDeleting] = useState<number | null>(null) // items done so far, while a deletion runs
  const [measured, setMeasured] = useState<{ key: string; bytes: number } | null>(null)
  const dialog = useRef<HTMLDialogElement>(null)

  // A new scan starts from a clean slate (PRD RF3, RF9).
  const scanning = h.status.phase === 'scanning'
  const clearSelection = () => {
    setSelected(new Set())
    setResult(null)
  }
  const rescan = () => {
    clearSelection()
    h.rescan()
  }
  const chooseRoot = async () => {
    if (await h.chooseRoot()) clearSelection()
  }

  const sorted = useMemo(() => [...projects].sort((a, b) => total(b, mode) - total(a, mode)), [projects, mode])
  const all = sorted.flatMap((p) => p.artifacts)
  const grand = all.reduce((s, a) => s + size(a, mode), 0)
  const pickedRows = sorted.flatMap((p) => p.artifacts.filter((a) => selected.has(a.id)).map((a) => ({ p, a })))
  const picked = pickedRows.map((r) => r.a)
  // Deletion always frees the real number. The backend measures the selection exactly (shared
  // hardlinks, RF19); until it answers, the local sum stands in.
  const key = picked.map((a) => a.id).sort().join(',')
  const { measure } = h
  useEffect(() => {
    if (!key) return
    const t = setTimeout(() => measure(key.split(',')).then((bytes) => setMeasured({ key, bytes }), () => {}), 150)
    return () => clearTimeout(t)
  }, [key, measure])
  const pickedBytes = measured?.key === key ? measured.bytes : picked.reduce((s, a) => s + a.real, 0)
  const pickedActive = sorted.filter((p) => isActive(p) && p.artifacts.some((a) => selected.has(a.id)))
  const real = mode === 'apparent' ? ' reais' : '' // flag real figures while the screen shows apparent ones

  const toggle = (ids: string[], on: boolean) =>
    setSelected((s) => {
      const n = new Set(s)
      ids.forEach((id) => (on ? n.add(id) : n.delete(id)))
      return n
    })

  const clean = async () => {
    setDeleting(0)
    try {
      setResult(await h.remove(picked.map((a) => a.id), setDeleting))
    } catch (e) {
      setResult({ freed: 0, refused: [{ label: 'Exclusão', reason: String(e) }] })
    }
    setSelected(new Set())
    setDeleting(null)
    dialog.current?.close()
  }

  return (
    <div className="grid h-full grid-cols-[minmax(0,1fr)] grid-rows-[56px_minmax(240px,46%)_1fr_84px]">
      <header className="flex items-center gap-5 border-b border-rule px-8">
        <h1 className="flex items-center gap-2.5 text-[19px] font-bold tracking-[0.01em]">
          <Mark /> Headroom
        </h1>
        <button onClick={chooseRoot} className="flex items-center gap-2 rounded-md border border-rule px-2.5 py-1 text-[14px] hover:border-ink" title="Escolher outra pasta raiz">
          <Folder /> {h.root || '~'}
        </button>
        {!h.live && <span className="rounded border border-rule px-2 py-0.5 text-[12px] font-medium tracking-[0.04em] text-graphite uppercase">Dados de exemplo</span>}
        <div role="radiogroup" aria-label="Tamanho" className="ml-auto flex overflow-hidden rounded-md border border-ink">
          {(['real', 'apparent'] as const).map((m) => (
            <button key={m} role="radio" aria-checked={mode === m} onClick={() => setMode(m)}
              className={`px-3 py-1.5 text-[13px] font-medium ${mode === m ? 'bg-ink text-paper' : 'hover:bg-paper-2'}`}>
              {m === 'real' ? 'Real' : 'Aparente'}
            </button>
          ))}
        </div>
        <button onClick={rescan} className="rounded-md border border-ink px-3.5 py-1.5 text-[14px] font-semibold hover:bg-paper-2">Escanear</button>
      </header>

      <section className="flex min-h-0 flex-col px-8 pt-4 pb-3">
        <div className="mb-2 flex items-baseline gap-3">
          <span className="text-[15px]"><b className="text-[16px]">{fmt(grand)}</b> recuperáveis em {sorted.length} projetos</span>
          <span className="text-[12px] font-medium tracking-[0.06em] text-graphite uppercase">Planta do disco · área = espaço {mode === 'real' ? 'real' : 'aparente'}</span>
          <span role="status" aria-live="polite" className="ml-auto text-[13px] text-graphite">{statusText(h.status)}</span>
        </div>
        {h.status.phase === 'error' ? (
          <div className="grid flex-1 place-items-center border-4 border-ink px-6 text-center">{h.status.message}</div>
        ) : sorted.length ? (
          <Plan projects={sorted} mode={mode} selected={selected} hover={hover} setHover={setHover} toggle={toggle} />
        ) : (
          <div className="grid flex-1 place-items-center border-4 border-ink text-graphite">{scanning ? 'Procurando artefatos…' : 'Nada para limpar nesta pasta.'}</div>
        )}
      </section>

      <section className="min-h-0 overflow-auto px-8">
        <table className="w-full border-collapse text-[14px]">
          <thead className="sticky top-0 bg-paper">
            <tr className="text-left text-[12px] font-semibold tracking-[0.06em] text-graphite uppercase">
              <th className="w-10 border-b-[1.5px] border-ink py-2 pl-2" />
              <th className="border-b-[1.5px] border-ink py-2">Artefato</th>
              <th className="border-b-[1.5px] border-ink py-2">Regra</th>
              <th className="border-b-[1.5px] border-ink py-2">Modificado</th>
              <th className="border-b-[1.5px] border-ink py-2 pr-2 text-right">Tamanho {mode === 'real' ? 'real' : 'aparente'}</th>
            </tr>
          </thead>
          {sorted.map((p) => {
            const ids = p.artifacts.map((a) => a.id)
            const n = ids.filter((id) => selected.has(id)).length
            return (
              <tbody key={p.id}>
                <tr>
                  <td className="border-b border-rule pt-4 pb-1.5 pl-2">
                    <input type="checkbox" aria-label={`Selecionar tudo de ${p.name}`} checked={n === ids.length}
                      ref={(el) => { if (el) el.indeterminate = n > 0 && n < ids.length }}
                      onChange={(e) => toggle(ids, e.target.checked)} />
                  </td>
                  <td colSpan={3} className="border-b border-rule pt-4 pb-1.5">
                    <span className="font-bold tracking-[0.04em] uppercase">{p.name}</span>
                    <span className="ml-3 text-[13px] text-graphite">{p.path}</span>
                    {isActive(p) && <Stamp className="ml-3">Ativo · {ago(p.days)}</Stamp>}
                  </td>
                  <td className="border-b border-rule pt-4 pr-2 pb-1.5 text-right font-bold">{fmt(total(p, mode))}</td>
                </tr>
                {p.artifacts.map((a) => (
                  <tr key={a.id} onMouseEnter={() => setHover(a.id)} onMouseLeave={() => setHover(null)}
                    className={hover === a.id ? 'bg-paper-2' : ''}>
                    <td className="border-b border-paper-2 py-2 pl-2">
                      <input type="checkbox" aria-label={`Selecionar ${a.rel} de ${p.name}`} checked={selected.has(a.id)}
                        onChange={(e) => toggle([a.id], e.target.checked)} />
                    </td>
                    <td className="border-b border-paper-2 py-2 font-medium">{a.rel}</td>
                    <td className="border-b border-paper-2 py-2 text-graphite">{a.rule}</td>
                    <td className="border-b border-paper-2 py-2 text-graphite">{ago(a.days)}</td>
                    <td className="border-b border-paper-2 py-2 pr-2 text-right">
                      {fmt(size(a, mode))}
                      {mode === 'real' && a.apparent > a.real * 1.5 && (
                        <span className="ml-1.5 text-[13px] text-graphite" title="Hardlinks compartilhados com o store do pnpm não liberam espaço">de {fmt(a.apparent)}</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            )
          })}
        </table>
      </section>

      <footer className="flex items-center gap-10 border-t-[1.5px] border-ink px-8">
        {result && !picked.length ? (
          <p role="status" className="line-clamp-2 flex-1 text-[15px]" title={result.refused.map((r) => `${r.label}: ${r.reason}`).join('\n')}>
            <b>{fmt(result.freed)} liberados.</b>{' '}
            {result.refused.length ? (
              <span>
                {result.refused.length} {result.refused.length === 1 ? 'não apagada' : 'não apagadas'}:{' '}
                <span className="text-graphite">{result.refused.map((r) => `${r.label} (${r.reason})`).join('; ')}</span>
              </span>
            ) : (
              <span className="text-graphite">Tudo volta com npm install ou cargo build quando você precisar.</span>
            )}
          </p>
        ) : (
          <DimensionLine part={pickedBytes} whole={all.reduce((s, a) => s + a.real, 0)} />
        )}
        <button disabled={!picked.length} onClick={() => dialog.current?.showModal()}
          className="shrink-0 rounded-md bg-ink px-5 py-3 text-[15px] font-semibold text-paper hover:bg-[#333] disabled:bg-rule disabled:text-graphite">
          {picked.length ? `Limpar ${fmt(pickedBytes)}${real}` : 'Selecione o que limpar'}
        </button>
      </footer>

      <dialog ref={dialog} className="m-auto w-[480px] rounded-lg border-[1.5px] border-ink bg-paper p-0 text-ink">
        <div className="p-6">
          <h2 className="text-[20px] font-bold">Apagar {picked.length} {picked.length === 1 ? 'pasta' : 'pastas'} de vez?</h2>
          <p className="mt-2 text-[15px] text-graphite">Libera {fmt(pickedBytes)} de espaço real. Não vai para a lixeira. Cada pasta foi validada pela regra e volta com o build ou install do projeto.</p>
          <div className="mt-4 flex justify-between text-[12px] font-semibold tracking-[0.06em] text-graphite uppercase"><span>Pasta</span><span>Espaço real</span></div>
          <ul className="mt-1 max-h-48 overflow-auto border-y border-rule py-2 text-[14px]">
            {pickedRows.map(({ p, a }) => (
              <li key={a.id} className="flex justify-between gap-4 py-1"><span className="truncate">{p.name}/{a.rel}</span><span className="shrink-0">{fmt(a.real)}</span></li>
            ))}
          </ul>
          {pickedActive.length > 0 && (
            <p className="mt-3 text-[14px]">
              <Stamp className="mr-2">Ativo</Stamp>
              {pickedActive.map((p) => p.name).join(', ')} {pickedActive.length === 1 ? 'foi modificado' : 'foram modificados'} nos últimos {ACTIVE_DAYS} dias. O próximo build vai demorar mais.
            </p>
          )}
        </div>
        <div className="flex justify-end gap-3 border-t border-rule px-6 py-4">
          <button autoFocus disabled={deleting !== null} onClick={() => dialog.current?.close()} className="rounded-md border border-ink px-4 py-2 font-semibold hover:bg-paper-2 disabled:border-rule disabled:text-graphite">Cancelar</button>
          <button disabled={deleting !== null} onClick={clean} className="rounded-md bg-brick px-4 py-2 font-semibold text-paper hover:bg-[#74210f] disabled:opacity-80">
            {deleting !== null ? `Apagando ${Math.min(deleting + 1, picked.length)} de ${picked.length}…` : `Apagar ${fmt(pickedBytes)}`}
          </button>
        </div>
      </dialog>
    </div>
  )
}

function statusText(s: Status) {
  if (s.phase === 'scanning') return `Varrendo… ${s.dirs.toLocaleString('pt-BR')} pastas lidas`
  if (s.phase === 'error') return 'A varredura falhou'
  const secs = (s.millis / 1000).toLocaleString('pt-BR', { maximumFractionDigits: 1 })
  const skipped = s.skipped ? ` · ${s.skipped} ${s.skipped === 1 ? 'pasta' : 'pastas'} sem permissão` : ''
  return `${s.count} ${s.count === 1 ? 'artefato' : 'artefatos'} em ${secs} s${skipped}`
}

// Rough text widths for Barlow, used only to decide whether a label fits in a room.
const widthOf = (s: string, px: number, caps = false) => s.length * px * (caps ? 0.66 : 0.52)

function Plan(props: {
  projects: Project[]; mode: Mode; selected: Set<string>; hover: string | null
  setHover: (id: string | null) => void; toggle: (ids: string[], on: boolean) => void
}) {
  const { projects, mode, selected, hover, setHover, toggle } = props
  const box = useRef<HTMLDivElement>(null)
  const [dim, setDim] = useState({ w: 0, h: 0 })
  useLayoutEffect(() => {
    const ro = new ResizeObserver(([e]) => setDim({ w: e.contentRect.width, h: e.contentRect.height }))
    ro.observe(box.current!)
    return () => ro.disconnect()
  }, [])

  // A real home folder holds hundreds of projects. The plan draws the big ones as rooms and pools the
  // rest into one "others" room; every project is still in the list below.
  const MAX_ROOMS = 24, MIN_SHARE = 0.01
  const grand = projects.reduce((s, p) => s + total(p, mode), 0)
  const shown = projects.filter((p, i) => i < MAX_ROOMS && total(p, mode) >= grand * MIN_SHARE)
  const rest = projects.slice(shown.length)
  const restBytes = rest.reduce((s, p) => s + total(p, mode), 0)
  const REST = '\0rest'
  const rooms = squarify(
    [...shown.map((p) => ({ id: p.id, value: total(p, mode) })), ...(rest.length ? [{ id: REST, value: restBytes }] : [])],
    0, 0, dim.w, dim.h,
  )
  const WALL = 1.5 // half of the 3px wall between rooms

  // Every room is lettered: name over area when it fits, otherwise a key number explained below the plan.
  const keyed: { n: number; p: Project }[] = []
  const lettering = (p: Project, w: number, h: number) => {
    const big = w > 220 && h > 120
    const area = fmt(total(p, mode))
    const areaPx = big ? 24 : 16
    const nameW = widthOf(p.name, 13, true) + (isActive(p) ? 56 : 0)
    const labelH = 18 + areaPx * 1.15
    if (w - 20 >= Math.max(nameW, widthOf(area, areaPx)) && h >= labelH + 12) return { kind: 'full' as const, area, areaPx, labelH }
    const n = keyed.length + 1
    keyed.push({ n, p })
    return { kind: 'key' as const, n, area, fitsArea: w - 16 >= 24 + widthOf(area, 13) && h >= 26, tiny: w < 30 || h < 30, labelH: 22 }
  }

  // Rooms too small to hold their key tag get it as a callout: tag beside the room, leader line into it.
  const TAG = 11 // half the tag size plus margin
  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi)
  const callouts: { n: number; active: boolean; cx: number; cy: number; tx: number; ty: number }[] = []

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 border-4 border-ink bg-ink">
        <div ref={box} className="relative h-full w-full">
          {rooms.map((r) => {
            const w = r.w - WALL * 2, h = r.h - WALL * 2
            if (r.id === REST) {
              const text = `+${rest.length} ${rest.length === 1 ? 'projeto' : 'projetos'}`
              return (
                <div key={r.id} title={`${text} menores · ${fmt(restBytes)} · estão todos na lista`}
                  className="absolute overflow-hidden bg-paper-2" style={{ left: r.x + WALL, top: r.y + WALL, width: w, height: h }}>
                  {w > 96 && h > 44 && (
                    <span className="absolute top-2 left-2 whitespace-nowrap">
                      <span className="block text-[13px] leading-[18px] font-bold tracking-[0.05em] uppercase">{text}</span>
                      <span className="block text-[16px] leading-[1.15] font-semibold">{fmt(restBytes)}</span>
                    </span>
                  )}
                </div>
              )
            }
            const p = projects.find((x) => x.id === r.id)!
            const label = lettering(p, w, h)
            if (label.kind === 'key' && label.tiny) {
              // Neighbouring alcoves letter their bottom-left corner, so the callout keeps to the room's far end.
              let cx = r.x + r.w / 2, cy = r.y + r.h / 2, tx = cx, ty = cy
              if (h < 30) { cx = tx = r.x + r.w - 16; ty = r.y - 18 >= TAG ? r.y - 18 : r.y + r.h + 18 }
              else { cy = ty = r.y + 16; tx = r.x - 20 >= TAG ? r.x - 20 : r.x + r.w + 20 }
              callouts.push({ n: label.n, active: isActive(p), cx, cy, tx: clamp(tx, TAG, dim.w - TAG), ty: clamp(ty, TAG, dim.h - TAG) })
            }
            const alcoves = squarify(p.artifacts.map((a) => ({ id: a.id, value: size(a, mode) })), 0, 0, w, h)
            return (
              <div key={r.id} className="absolute overflow-hidden bg-ink" style={{ left: r.x + WALL, top: r.y + WALL, width: w, height: h }}>
                {alcoves.map((c) => (
                  <Alcove key={c.id} c={c} a={p.artifacts.find((x) => x.id === c.id)!} p={p} mode={mode}
                    on={selected.has(c.id)} hovered={hover === c.id} reserve={label.labelH + 10}
                    toggle={toggle} setHover={setHover} />
                ))}
                {!(label.kind === 'key' && label.tiny) && <span className="pointer-events-none absolute top-2 left-2 max-w-[calc(100%-16px)] rounded-sm bg-paper px-1.5 py-0.5 whitespace-nowrap">
                  {label.kind === 'full' ? (
                    <>
                      <span className="flex items-center gap-2 text-[13px] leading-[18px] font-bold tracking-[0.05em] uppercase">
                        {p.name} {isActive(p) && <Stamp>Ativo</Stamp>}
                      </span>
                      <span className="block leading-[1.15] font-semibold" style={{ fontSize: label.areaPx }}>{label.area}</span>
                    </>
                  ) : (
                    <span className="flex items-center gap-1.5 text-[13px] leading-[18px]">
                      <KeyTag n={label.n} active={isActive(p)} />
                      {label.fitsArea && <b className="font-semibold">{label.area}</b>}
                    </span>
                  )}
                </span>}
              </div>
            )
          })}
          {callouts.length > 0 && (
            <svg className="pointer-events-none absolute inset-0 h-full w-full overflow-visible" aria-hidden="true">
              {callouts.map((c) => (
                <g key={c.n}>
                  <line x1={c.tx} y1={c.ty} x2={c.cx} y2={c.cy} stroke="var(--color-ink)" strokeWidth="1.25" />
                  <circle cx={c.cx} cy={c.cy} r="2" fill="var(--color-ink)" stroke="var(--color-paper)" strokeWidth="1" />
                </g>
              ))}
            </svg>
          )}
          {callouts.map((c) => (
            <span key={c.n} className="pointer-events-none absolute" style={{ left: c.tx - 9, top: c.ty - 9 }}>
              <KeyTag n={c.n} active={c.active} />
            </span>
          ))}
        </div>
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-x-5 gap-y-1 text-[13px]">
        <span className="flex items-center gap-2"><i className="hatch inline-block h-3.5 w-6 border border-draft" /> Selecionado para limpar</span>
        <span className="flex items-center gap-2"><i className="poche-swatch inline-block h-3.5 w-6 border border-ink" /> Ativo · últimos {ACTIVE_DAYS} dias</span>
        {keyed.map(({ n, p }) => (
          <span key={n} className="flex items-center gap-1.5 whitespace-nowrap"><KeyTag n={n} active={isActive(p)} /> {p.name} <span className="text-graphite">{fmt(total(p, mode))}</span></span>
        ))}
        {rest.length > 0 && (
          <span className="flex items-center gap-2 whitespace-nowrap"><i className="inline-block h-3.5 w-6 border border-ink bg-paper-2" /> +{rest.length} {rest.length === 1 ? 'projeto menor' : 'projetos menores'} <span className="text-graphite">{fmt(restBytes)} · na lista</span></span>
        )}
      </div>
    </div>
  )
}

function Alcove(props: {
  c: Rect; a: Artifact; p: Project; mode: Mode; on: boolean; hovered: boolean; reserve: number
  toggle: (ids: string[], on: boolean) => void; setHover: (id: string | null) => void
}) {
  const { c, a, p, mode, on, hovered, reserve, toggle, setHover } = props
  const cw = Math.max(c.w - 1, 0), ch = Math.max(c.h - 1, 0)
  const underLabel = c.x < 2 && c.y < 2 // the room lettering sits in this alcove's top-left corner
  const showLabel = cw > 84 && ch > (underLabel ? reserve + 40 : 40)
  return (
    <button aria-pressed={on} aria-label={`${a.rel} de ${p.name}, ${fmt(size(a, mode))}`}
      title={`${p.name}/${a.rel} · ${fmt(size(a, mode))}`}
      onClick={() => toggle([a.id], !on)} onMouseEnter={() => setHover(a.id)} onMouseLeave={() => setHover(null)}
      className={`absolute flex flex-col items-start justify-end overflow-hidden px-2 pb-1.5 text-left ${on ? 'hatch' : isActive(p) ? 'poche bg-paper' : 'bg-paper'} ${hovered ? 'outline-2 -outline-offset-4 outline-ink' : ''}`}
      style={{ left: c.x + 0.5, top: c.y + 0.5, width: cw, height: ch }}>
      {showLabel && (
        <span className="rounded-sm bg-paper px-1 leading-tight">
          <span className="block text-[12px] text-graphite">{a.rel}</span>
          <span className="block text-[14px] font-semibold">{fmt(size(a, mode))}</span>
        </span>
      )}
    </button>
  )
}

function DimensionLine({ part, whole }: { part: number; whole: number }) {
  const pct = whole ? (part / whole) * 100 : 0
  // Each measure sits centred over its own span; clamp so a short span's label stays on screen.
  const mid = (a: number, b: number) => Math.min(Math.max((a + b) / 2, 9), 91)
  return (
    <div className="relative h-14 flex-1" aria-label={`${fmt(part)} selecionados de ${fmt(whole)} recuperáveis`}>
      <div className="absolute inset-x-0 top-[25px] border-t border-ink" />
      <div className="absolute top-[24px] left-0 h-[3px] bg-draft transition-[width] duration-300 ease-out" style={{ width: `${pct}%` }} />
      {[0, pct, 100].map((x, i) => (
        <i key={i} className="absolute top-[18px] h-[15px] w-px rotate-45 bg-ink transition-[left] duration-300 ease-out" style={{ left: `${x}%` }} />
      ))}
      {pct > 0 && (
        <span className="absolute top-0 -translate-x-1/2 text-[13px] whitespace-nowrap transition-[left] duration-300 ease-out" style={{ left: `${mid(0, pct)}%` }}>
          <b>{fmt(part)}</b> selecionados
        </span>
      )}
      <span className="absolute top-0 -translate-x-1/2 text-[13px] whitespace-nowrap text-graphite transition-[left] duration-300 ease-out" style={{ left: `${mid(pct, 100)}%` }}>
        {fmt(whole - part)} ficam
      </span>
      <span className="absolute right-0 bottom-0 text-[12px] text-graphite">de {fmt(whole)} recuperáveis, em espaço real</span>
    </div>
  )
}

function Stamp({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return <span className={`inline-block border-[1.5px] border-ink bg-paper px-1.5 text-[11px] leading-[16px] font-bold tracking-[0.1em] uppercase ${className}`}>{children}</span>
}

function KeyTag({ n, active }: { n: number; active: boolean }) {
  return (
    <span className={`inline-grid h-[18px] min-w-[18px] place-items-center rounded-full border-[1.5px] border-ink px-1 text-[11px] font-bold ${active ? 'poche bg-paper' : 'bg-paper'}`}
      title={active ? 'Ativo' : undefined}>{n}</span>
  )
}

function Folder() {
  return (
    <svg width="16" height="14" viewBox="0 0 16 14" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M1 2.5h4.5l1.5 1.5H15v8.5H1z" />
    </svg>
  )
}

function Mark() {
  // The Headroom gauge: ink is what stays, blue is what gets freed, the dashed line is the headroom left.
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4.5" y="1.8" width="15" height="20.4" rx="3" fill="none" stroke="currentColor" strokeWidth="2.2" />
      <path d="M7.2 14.2q4.8 1.8 9.6 0V19H7.2z" fill="currentColor" />
      <path d="M7.2 9.8q4.8 1.8 9.6 0v2.6q-4.8 1.8-9.6 0z" fill="var(--color-draft)" />
      <path d="M7.6 6q4.4 1.5 8.8 0" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="1.4 1.1" />
    </svg>
  )
}
