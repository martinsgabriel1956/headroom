import assert from 'node:assert/strict'
import type { Project } from './data'
import { withUniqueNames } from './names.ts'

const p = (id: string): Project => ({ id, name: id.split('/').pop()!, path: id, days: Infinity, artifacts: [] })
const names = withUniqueNames([
  p('/h/vilazen/web/main'),
  p('/h/vilazen/api/main'),
  p('/h/interno/web/feat/controle-pedra'),
  p('/h/interno/api/feat/controle-pedra'),
  p('/h/editor'),
]).map((x) => x.name)
assert.deepEqual(names, ['web/main', 'api/main', 'web/feat/controle-pedra', 'api/feat/controle-pedra', 'editor'])
assert.deepEqual(withUniqueNames([p('sample')]).map((x) => x.name), ['sample'])
console.log('names ok')
