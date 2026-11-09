import assert from 'node:assert/strict'
import { squarify } from './treemap.ts'

const items = [6, 6, 4, 3, 2, 2, 1].map((value, i) => ({ id: String(i), value }))
const rects = squarify(items, 0, 0, 600, 400)

assert.equal(rects.length, items.length)
// Areas are proportional to values.
for (const r of rects) {
  const v = items.find((i) => i.id === r.id)!.value
  assert.ok(Math.abs(r.w * r.h - (v / 24) * 600 * 400) < 1e-6)
}
// Everything stays inside the box and nothing overlaps.
for (const a of rects) {
  assert.ok(a.x >= -1e-9 && a.y >= -1e-9 && a.x + a.w <= 600 + 1e-9 && a.y + a.h <= 400 + 1e-9)
  for (const b of rects) {
    if (a === b) continue
    const ox = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x)
    const oy = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y)
    assert.ok(ox <= 1e-6 || oy <= 1e-6, `${a.id} overlaps ${b.id}`)
  }
}
assert.deepEqual(squarify([{ id: 'x', value: 0 }], 0, 0, 10, 10), [])
console.log('treemap ok')
