import * as React from 'react'
import Button from '@mui/material/Button'

export default function ClipboardButton({ text }: { text: string }) {
  return (
    <Button onPointerDown={() => navigator.clipboard.writeText(text)}>
      Copy to Clipboard
    </Button>
  )
}
