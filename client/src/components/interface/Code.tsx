import * as React from 'react'
import ReactCodeMirror from '@uiw/react-codemirror'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'

import { useStore } from '@hooks/useStore'

interface Props {
  code: string
  setCode: (code: string) => void
  textMode?: boolean
}

export function Code({ code, setCode, textMode = false }: Props) {
  const darkMode = useStore((s) => s.darkMode)

  const extensions = React.useMemo(
    () => (textMode ? [json()] : [json(), linter(jsonParseLinter())]),
    [textMode],
  )

  return (
    <ReactCodeMirror
      key={darkMode.toString()}
      extensions={extensions}
      theme={darkMode ? 'dark' : 'light'}
      value={code}
      onUpdate={(value) => {
        if (value.docChanged) {
          setCode(value.state.doc.toString())
        }
      }}
    />
  )
}
