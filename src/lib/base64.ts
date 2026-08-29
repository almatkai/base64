function compact(value: string): string {
  return value.replace(/\s+/g, '')
}

function withoutPadding(value: string): string {
  return value.replace(/=+$/, '')
}

export function encodeBase64(value: string): string {
  return Buffer.from(value, 'utf8').toString('base64')
}

export function isValidBase64(value: string): boolean {
  const normalized = compact(value)
  if (!normalized || normalized.length % 4 === 1 || !/^[A-Za-z0-9+/]*={0,2}$/.test(normalized))
    return false
  try {
    const decoded = Buffer.from(normalized, 'base64')
    return withoutPadding(decoded.toString('base64')) === withoutPadding(normalized)
  } catch {
    return false
  }
}

export function decodeBase64(value: string): string {
  if (!isValidBase64(value)) throw new Error('Input is not valid Base64 text.')
  return Buffer.from(compact(value), 'base64').toString('utf8')
}
