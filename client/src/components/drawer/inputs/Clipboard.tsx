import * as React from 'react'
import Button from '@mui/material/Button'
import { Tooltip } from '@mui/material'

const https = !!navigator.clipboard

export default function ClipboardButton({ text }: { text: string }) {
  return (
    <Tooltip title={https ? '' : 'This requires a secure connection to use'}>
      <Button
        disabled={!https}
        onPointerDown={() => navigator.clipboard.writeText(text)}
      >
        Copy to Clipboard
      </Button>
    </Tooltip>
  )
}
