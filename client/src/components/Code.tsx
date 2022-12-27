import * as React from 'react'
import ReactCodeMirror from '@uiw/react-codemirror'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'

import { usePersist } from '@hooks/usePersist'
import { getData } from '@services/fetches'

interface Props {
  code: string
  setCode: (code: string) => void
  textMode?: boolean
}

export function Code({ code, setCode, textMode = false }: Props) {
  const darkMode = usePersist((s) => s.darkMode)

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
      onUpdate={async (value) => {
        if (value.docChanged) {
          const newValue = value.state.doc.toString()
          if (newValue.startsWith('http')) {
            const remoteValue = await getData<object>(newValue)
            setCode(JSON.stringify(remoteValue, null, 2))
          } else {
            setCode(newValue)
          }
        }
      }}
    />
  )
}
