import { Clipboard, Toast, closeMainWindow, showToast } from '@raycast/api'
import { encodeBase64 } from './lib/base64'

export default async function Command(): Promise<void> {
  try {
    const { text } = await Clipboard.read()
    if (!text) throw new Error('The clipboard does not contain text.')
    await Clipboard.copy(encodeBase64(text))
    await closeMainWindow()
    await showToast(Toast.Style.Success, 'Encoded text copied to clipboard')
  } catch (error) {
    await showToast(
      Toast.Style.Failure,
      'Could not encode clipboard',
      String((error as Error).message)
    )
  }
}
