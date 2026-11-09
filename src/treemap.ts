export type Rect = { id: string; x: number; y: number; w: number; h: number }

// Squarified treemap (Bruls et al.): lays items into the box keeping cells close to square.
export function squarify(items: { id: string; value: number }[], x: number, y: number, w: number, h: number): Rect[] {
  const total = items.reduce((s, i) => s + i.value, 0)
  if (total <= 0 || w <= 0 || h <= 0) return []
  const scale = (w * h) / total
  const rest = items
    .filter((i) => i.value > 0)
    .sort((a, b) => b.value - a.value)
    .map((i) => ({ id: i.id, area: i.value * scale }))
  const out: Rect[] = []

  while (rest.length) {
    const side = Math.min(w, h)
    const row = [rest.shift()!]
    while (rest.length && worst([...row, rest[0]], side) <= worst(row, side)) row.push(rest.shift()!)

    const thick = row.reduce((s, r) => s + r.area, 0) / side
    let off = 0
    for (const r of row) {
      const len = r.area / thick
      out.push(w >= h ? { id: r.id, x, y: y + off, w: thick, h: len } : { id: r.id, x: x + off, y, w: len, h: thick })
      off += len
    }
    if (w >= h) { x += thick; w -= thick } else { y += thick; h -= thick }
  }
  return out
}

function worst(row: { area: number }[], side: number) {
  const s = row.reduce((t, r) => t + r.area, 0)
  const max = Math.max(...row.map((r) => r.area))
  const min = Math.min(...row.map((r) => r.area))
  return Math.max((side * side * max) / (s * s), (s * s) / (side * side * min))
}
