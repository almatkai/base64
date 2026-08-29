import { Clipboard, Toast, closeMainWindow, showToast } from '@raycast/api'
import { decodeBase64, isValidBase64 } from './lib/base64'

export default async function Command(): Promise<void> {
  try {
    const { text } = await Clipboard.read()
    if (!text || !isValidBase64(text)) throw new Error('The clipboard is not valid Base64 text.')
    await Clipboard.copy(decodeBase64(text))
    await closeMainWindow()
    await showToast(Toast.Style.Success, 'Decoded text copied to clipboard')
  } catch (error) {
    await showToast(
      Toast.Style.Failure,
      'Could not decode clipboard',
      String((error as Error).message)
    )
  }
}
