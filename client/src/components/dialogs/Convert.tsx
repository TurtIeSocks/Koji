import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import {
  Box,
  FormControl,
  FormControlLabel,
  Switch,
  Typography,
} from '@mui/material'

import { ToConvert } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { Code } from '@components/Code'
import MultiOptions from '@components/drawer/inputs/MultiOptions'
import { safeParse } from '@services/utils'
import { convert } from '@services/fetches'

import BaseDialog from './Base'

export default function ConvertDialog() {
  const open = useStatic((s) => s.dialogs.convert)
  const polygonExportMode = usePersist((s) => s.polygonExportMode)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const [code, setCode] = React.useState('')
  const [converted, setConverted] = React.useState('')

  const convertCode = async (incoming: ToConvert) => {
    await convert(incoming, polygonExportMode, simplifyPolygons).then((res) => {
      if (typeof res === 'string') {
        setConverted(res)
      } else {
        setConverted(JSON.stringify(res, null, 2))
      }
    })
  }

  React.useEffect(() => {
    if (open) {
      setCode('')
      setConverted('')
    }
  }, [open])

  React.useEffect(() => {
    if (code) {
      const incoming = safeParse<ToConvert>(code)
      if (!incoming.error) {
        convertCode(incoming.value)
      }
    }
  }, [polygonExportMode, simplifyPolygons])

  return (
    <BaseDialog
      open={open}
      onClose={() =>
        useStatic.setState((prev) => ({
          dialogs: { ...prev.dialogs, convert: false },
        }))
      }
      title="Conversion Playground"
      Components={{
        Dialog: { maxWidth: 'xl' },
        DialogActions: {
          children: (
            <>
              <MultiOptions
                field="polygonExportMode"
                buttons={[
                  'array',
                  'multiArray',
                  'feature',
                  'featureCollection',
                  'struct',
                  'multiStruct',
                  'text',
                  'altText',
                  'poracle',
                ]}
                type="select"
              />
              <FormControl>
                <FormControlLabel
                  value={simplifyPolygons}
                  label="Simplify Polygons"
                  control={
                    <Switch
                      value={simplifyPolygons}
                      onChange={() =>
                        usePersist.setState((prev) => ({
                          simplifyPolygons: !prev.simplifyPolygons,
                        }))
                      }
                    />
                  }
                />
              </FormControl>
              <Box sx={{ flex: '1 1 auto' }} />
            </>
          ),
        },
      }}
    >
      <Grid2 container height="85vh" minWidth="85vw" alignItems="flex-start">
        <Grid2 xs={12} sm={6} textAlign="left">
          <Typography variant="h3" align="center" my={1}>
            Input
          </Typography>
          <Code
            code={code}
            setCode={async (newCode) => {
              const parsed = safeParse<ToConvert>(newCode)
              if (!parsed.error) {
                setCode(newCode)
                await convertCode(parsed.value)
              } else if (typeof parsed.error === 'string') {
                setConverted(parsed.error)
              }
            }}
          />
        </Grid2>
        <Grid2 xs={12} sm={6} textAlign="left">
          <Typography variant="h3" align="center" my={1}>
            Result
          </Typography>
          <Code>{converted}</Code>
        </Grid2>
      </Grid2>
    </BaseDialog>
  )
}
