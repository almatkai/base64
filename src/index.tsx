import {
  Action,
  ActionPanel,
  Clipboard,
  Color,
  Icon,
  List,
  getPreferenceValues,
} from '@raycast/api'
import { useEffect, useMemo, useState } from 'react'

import { decodeBase64, encodeBase64, isValidBase64 } from './lib/base64'

type Preferences = { defaultAction?: 'copyToClipboard' | 'pasteInApp' | 'openInBrowser' }

function isValidUrl(value: string): boolean {
  try {
    const url = new URL(value)
    return url.protocol === 'http:' || url.protocol === 'https:'
  } catch {
    return false
  }
}

function preview(value: string): string {
  return value.replace(/\r/g, '\\r').replace(/\n/g, '\\n').replace(/\t/g, '\\t')
}

function ResultActions({ value }: { value: string }) {
  const preferences = getPreferenceValues<Preferences>()
  const actions = {
    copyToClipboard: (
      <Action.CopyToClipboard key="copy" title="Copy to Clipboard" content={value} />
    ),
    pasteInApp: <Action.Paste key="paste" title="Paste in Active App" content={value} />,
    openInBrowser: isValidUrl(value) ? (
      <Action.OpenInBrowser key="browser" title="Open URL in Browser" url={value} />
    ) : null,
  }
  const order: Array<keyof typeof actions> = ['copyToClipboard', 'pasteInApp', 'openInBrowser']
  const preferred = preferences.defaultAction ?? 'copyToClipboard'
  const sorted = [preferred, ...order.filter((key) => key !== preferred)]

  return <ActionPanel>{sorted.map((key) => actions[key])}</ActionPanel>
}

export default function Command() {
  const [input, setInput] = useState('')
  const encoded = useMemo(() => (input ? encodeBase64(input) : ''), [input])
  const decoded = useMemo(
    () => (input && isValidBase64(input) ? decodeBase64(input) : null),
    [input]
  )

  useEffect(() => {
    void Clipboard.read().then(({ text }) => {
      if (text) setInput(text)
    })
  }, [])

  return (
    <List onSearchTextChange={setInput} searchBarPlaceholder="Text to encode or decode...">
      {input ? (
        <List.Section title={`Input: ${preview(input)}`}>
          <List.Item
            icon={{ source: Icon.CodeBlock, tintColor: Color.Blue }}
            title="Encode"
            subtitle={encoded}
            actions={<ResultActions value={encoded} />}
          />
          {decoded !== null ? (
            <List.Item
              icon={{ source: Icon.Code, tintColor: Color.Purple }}
              title="Decode"
              subtitle={preview(decoded)}
              actions={<ResultActions value={decoded} />}
            />
          ) : null}
        </List.Section>
      ) : null}
      <List.EmptyView
        icon={{ source: Icon.QuestionMarkCircle, tintColor: Color.SecondaryText }}
        title="Nothing to Encode or Decode"
        description="Copy text to the clipboard or type into the search field."
      />
    </List>
  )
}
