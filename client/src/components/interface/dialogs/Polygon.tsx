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
interface ExportAll extends Props {
  mode: 'exportAll'
  feature: FeatureCollection
}

export function ExportPolygon(props: Import): JSX.Element
export function ExportPolygon(props: Export): JSX.Element
export function ExportPolygon(props: ExportAll): JSX.Element
export default function ExportPolygon({
  mode,
  open,
  setOpen,
  feature,
}: Import | Export | ExportAll): JSX.Element {
  const polygonExportMode = useStore((s) => s.polygonExportMode)
  const setStore = useStore((s) => s.setStore)

  const setStatic = useStatic((s) => s.setStatic)
  const [code, setCode] = React.useState('')
  const [error, setError] = React.useState('')
  const [tempGeojson, setTempGeojson] = React.useState<FeatureCollection>()

  React.useEffect(() => {
    if (mode === 'export' || mode === 'exportAll') {
      ;(async () => {
        switch (polygonExportMode) {
          default:
            return convert(feature, polygonExportMode)
        }
      })().then((newCode) => {
        if (typeof newCode === 'string') {
          setCode(newCode)
        } else {
          setCode(
            JSON.stringify(
              mode === 'export' &&
                polygonExportMode === 'poracle' &&
                Array.isArray(newCode)
                ? newCode[0]
                : newCode,
              null,
              2,
            ),
          )
        }
      })
    }
  }, [polygonExportMode])

  React.useEffect(() => {
    if (mode === 'import' && code) {
      ;(async () => {
        try {
          const cleanCode = code.trim()
          const parsed: ToConvert =
            cleanCode.startsWith('{') || cleanCode.startsWith('[')
              ? JSON.parse(
                  cleanCode.endsWith(',')
                    ? cleanCode.substring(0, cleanCode.length - 1)
                    : cleanCode,
                )
              : cleanCode
          const geojson = await convert<FeatureCollection>(
            parsed,
            'featureCollection',
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
  }, [polygonExportMode, code])
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
        action={() => {
          setOpen('')
          setCode('')
        }}
      >
        {mode === 'export' || mode === 'exportAll'
          ? 'Export Polygon'
          : 'Import Polygon'}
      </DialogHeader>
      <DialogContent sx={{ width: '90vw', height: '60vh', overflow: 'auto' }}>
        <Code
          code={code}
          setCode={setCode}
          textMode={
            mode === 'export' || mode === 'exportAll'
              ? polygonExportMode === 'text' || polygonExportMode === 'altText'
              : !code.startsWith('{') && !code.startsWith('[')
          }
        />
      </DialogContent>
      <DialogActions>
        {(mode === 'export' || mode === 'exportAll') && (
          <BtnGroup
            field="polygonExportMode"
            value={polygonExportMode}
            setValue={setStore}
            buttons={[
              'featureCollection',
              'feature',
              'array',
              'struct',
              'text',
              'altText',
              'poracle',
            ]}
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
              setStatic('geojson', (geo) => ({
                ...geo,
                features: [...geo.features, ...tempGeojson.features],
              }))
            }
          }}
        >
          {mode === 'export' || mode === 'exportAll' ? 'Close' : 'Import'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
