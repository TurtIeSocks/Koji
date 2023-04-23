import * as React from 'react'
import Button, { type ButtonProps } from '@mui/material/Button'

export default function DownloadBtn({
  data,
  ...props
}: ButtonProps & { data: object | string }) {
  return (
    <Button
      variant="outlined"
      color="success"
      {...props}
      onClick={() => {
        const el = document.createElement('a')
        el.setAttribute(
          'href',
          `data:application/json;chartset=utf-8,${encodeURIComponent(
            typeof data === 'string' ? data : JSON.stringify(data),
          )}`,
        )
        el.setAttribute('download', 'geojson.json')
        el.style.display = 'none'
        document.body.appendChild(el)
        el.click()
        document.body.removeChild(el)
      }}
    >
      Download
    </Button>
  )
}
