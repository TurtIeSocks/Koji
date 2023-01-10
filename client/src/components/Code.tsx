import * as React from 'react'
import ReactCodeMirror, { ReactCodeMirrorProps } from '@uiw/react-codemirror'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'

import { usePersist } from '@hooks/usePersist'
import { getData } from '@services/fetches'
import Typography from '@mui/material/Typography'
import { Box } from '@mui/material'

interface EditProps extends ReactCodeMirrorProps {
  code?: string
  setCode: (code: string) => void
  textMode?: boolean
  children?: string
  maxHeight?: string
}
interface ReadProps extends Partial<EditProps> {
  children: string
}

export function Code({
  code,
  setCode,
  textMode = false,
  children,
  ...rest
}: EditProps | ReadProps) {
  const darkMode = usePersist((s) => s.darkMode)

  const extensions = React.useMemo(
    () => (textMode ? [json()] : [json(), linter(jsonParseLinter())]),
    [textMode],
  )

  return (
    <Box py={3}>
      <ReactCodeMirror
        key={darkMode.toString()}
        extensions={extensions}
        theme={darkMode ? 'dark' : 'light'}
        value={children ?? code ?? ''}
        onUpdate={async (value) => {
          if (value.docChanged) {
            if (setCode) {
              const newValue = value.state.doc.toString()
              if (newValue.startsWith('http')) {
                const remoteValue = await getData<object>(newValue)
                setCode(JSON.stringify(remoteValue, null, 2))
              } else {
                setCode(newValue)
              }
            }
          }
        }}
        {...rest}
      />
      <Typography variant="caption" color="grey">
        You can also try entering a url for a remote JSON, Kōji will attempt to
        fetch and parse it.
      </Typography>
    </Box>
  )
}
