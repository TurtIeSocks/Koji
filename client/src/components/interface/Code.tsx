import * as React from 'react'
import ReactCodeMirror from '@uiw/react-codemirror'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'

interface Props {
  code: string
  setCode: (code: string) => void
}

export function Code({ code, setCode }: Props) {
  const extensions = React.useMemo(
    () => [json(), linter(jsonParseLinter())],
    [],
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
