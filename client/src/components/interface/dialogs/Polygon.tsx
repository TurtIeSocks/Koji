import * as React from 'react'
import { Dialog, DialogContent, DialogActions, Button } from '@mui/material'

import type { Feature, FeatureCollection } from 'geojson'

import type { ToConvert } from '@assets/types'
import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'
import { convert } from '@services/fetches'

import DialogHeader from './Header'
import { Code } from '../Code'
import BtnGroup from '../drawer/inputs/BtnGroup'

interface Props {
  open: string
  setOpen: (open: string) => void
}
interface Import extends Props {
  mode: 'import'
  feature: FeatureCollection
}
interface Export extends Props {
  mode: 'export'
  feature: Feature
}

export function ExportPolygon(props: Import): JSX.Element
export function ExportPolygon(props: Export): JSX.Element
export default function ExportPolygon({
  mode,
  open,
  setOpen,
  feature,
}: Import | Export): JSX.Element {
  const polygonExportMode = useStore((s) => s.polygonExportMode)
  const setStore = useStore((s) => s.setStore)

  const setStatic = useStatic((s) => s.setStatic)
  const [code, setCode] = React.useState('')
  const [error, setError] = React.useState('')
  const [tempGeojson, setTempGeojson] = React.useState<FeatureCollection>()

  React.useEffect(() => {
    if (mode === 'export') {
      ;(async () => {
        switch (polygonExportMode) {
          case 'geojson':
            setCode(JSON.stringify(feature, null, 2))
            break
          case 'array':
            setCode(await convert(feature, 'array'))
            break
          case 'object':
            setCode(await convert(feature, 'struct'))
            break
          case 'text':
            setCode(await convert(feature, 'text'))
            break
          default:
            break
        }
      })()
    }
  }, [polygonExportMode, code])

  React.useEffect(() => {
    if (mode === 'import' && code) {
      ;(async () => {
        try {
          const parsed: ToConvert =
            code.startsWith('{') || code.startsWith('[')
              ? JSON.parse(code)
              : code
          const geojson: FeatureCollection = await convert<FeatureCollection>(
            parsed,
            'feature_collection',
            true,
          )
          if (geojson.type === 'FeatureCollection') {
            setTempGeojson(geojson)
          }
          setError('')
        } catch (e) {
          if (e instanceof Error) {
            setError(e.message)
          }
        }
      })()
    }
  }, [code])
  return (
    <Dialog
      open={open === 'polygon'}
      onClose={() => {
        setOpen('')
        setCode('')
      }}
      maxWidth="xl"
    >
      <DialogHeader
        title={mode === 'export' ? 'Export Polygon' : 'Import Polygon'}
        action={() => {
          setOpen('')
          setCode('')
        }}
      />
      <DialogContent sx={{ width: '90vw', height: '60vh', overflow: 'auto' }}>
        <Code
          code={code}
          setCode={setCode}
          textMode={
            mode === 'export'
              ? polygonExportMode === 'text'
              : !code.startsWith('{') && !code.startsWith('[')
          }
        />
      </DialogContent>
      <DialogActions>
        {mode === 'export' && (
          <BtnGroup
            field="polygonExportMode"
            value={polygonExportMode}
            setValue={setStore}
            buttons={['array', 'geojson', 'object', 'text']}
          />
        )}
        <Button onClick={() => navigator.clipboard.writeText(code)}>
          Copy to Clipboard
        </Button>
        <Button
          disabled={!!error}
          onClick={() => {
            setOpen('')
            setCode('')
            if (mode === 'import' && tempGeojson) {
              setStatic('geojson', tempGeojson)
            }
          }}
        >
          {mode === 'export' ? 'Close' : 'Import'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
