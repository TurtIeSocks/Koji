import * as React from 'react'
import ReactCodeMirror from '@uiw/react-codemirror'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'
import { UseStore } from '@hooks/useStore'

interface Props {
  code: string
  setCode: (code: string) => void
  mode: UseStore['polygonExportMode']
}

export function Code({ code, setCode, mode }: Props) {
  const extensions = React.useMemo(
    () => (mode === 'text' ? [json()] : [json(), linter(jsonParseLinter())]),
    [mode],
  )

  return (
    <ReactCodeMirror
      extensions={extensions}
      theme="light"
      value={code}
      onUpdate={(value) => {
        if (value.docChanged) {
          setCode(value.state.doc.toString())
        }
      }}
    />
  )
}
