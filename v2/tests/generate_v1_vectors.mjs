// Test data generator only, not an extension runtime or process loader.
// Run from repo root: node --import tsx v2/tests/generate_v1_vectors.mjs
import { writeFileSync } from 'node:fs'
import { encodeBase64, decodeBase64, isValidBase64 } from '../../src/lib/base64.ts'

const inputs = new Set(['', '=', '==', '===', 'Zg', 'Zg=', 'Zg==', 'Zm9v=', 'Zm9v==', 'Zm8==', 'Zh', 'Zh=', 'Zh==', 'Zm9', '\ufeffZg==', '\u0085Zg==', '7aCA', '4oI=', '/w=='])
for (const text of ['hello', 'Сәлем 🌍', '\r\n\t\0', 'https://example.com/日本']) {
  const encoded = encodeBase64(text)
  for (let padding = 0; padding <= 3; padding++) inputs.add(encoded.replace(/=+$/, '') + '='.repeat(padding))
}
// Deterministic oracle from the untouched V1, including malformed UTF-8.
let seed = 0x71c0ffee
function random() { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed }
for (let i = 0; i < 256; i++) {
  inputs.add(Buffer.from([i]).toString('base64'))
  const bytes = Buffer.from(Array.from({length: 1 + random() % 16}, () => random() >>> 24))
  const encoded = bytes.toString('base64')
  inputs.add(encoded)
  inputs.add(encoded.replace(/=+$/, '') + '='.repeat(random() % 4))
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=_-!'
  inputs.add(Array.from({length: 1 + random() % 12}, () => alphabet[random() % alphabet.length]).join(''))
}
const vectors = [...inputs].map(input => {
  const valid = isValidBase64(input)
  return { input, encoded: encodeBase64(input), valid, decoded: valid ? decodeBase64(input) : null }
})
writeFileSync(new URL('./fixtures/v1-vectors.json', import.meta.url), JSON.stringify({node: process.version, vectors}, null, 2) + '\n')
console.log(`Wrote ${vectors.length} V1 oracle vectors`)
