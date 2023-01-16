import * as React from 'react'
import { Dialog, DialogContent, DialogActions, Button } from '@mui/material'

import type { Feature, FeatureCollection } from 'geojson'

import type { ToConvert } from '@assets/types'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'
import { convert } from '@services/fetches'
import MultiOptions from '@components/drawer/inputs/MultiOptions'

import DialogHeader from './Header'
import { Code } from '../Code'

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
  const polygonExportMode = usePersist((s) => s.polygonExportMode)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const add = useShapes((s) => s.setters.add)

  const [code, setCode] = React.useState('')
  const [error, setError] = React.useState('')
  const [tempGeojson, setTempGeojson] = React.useState<FeatureCollection>({
    type: 'FeatureCollection',
    features: [],
  })

  React.useEffect(() => {
    if (mode === 'export' || mode === 'exportAll') {
      ;(async () => {
        switch (polygonExportMode) {
          default:
            return convert(feature, polygonExportMode, simplifyPolygons)
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
  }, [polygonExportMode, open])

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
            simplifyPolygons,
          )
          if (geojson.type === 'FeatureCollection') {
            setTempGeojson({
              ...geojson,
              features: geojson.features.map((f) => ({
                ...f,
                id: f.id ?? `${f.properties?.__name}${f.properties?.__type}`,
              })),
            })
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
        )}
        {/* <Button
          onClick={async () => {
          }}
        >
          Combine
        </Button> */}
        <Button onPointerDown={() => navigator.clipboard.writeText(code)}>
          Copy to Clipboard
        </Button>
        <Button
          disabled={!!error}
          onClick={() => {
            setOpen('')
            setCode('')
            if (mode === 'import' && tempGeojson) {
              add(tempGeojson.features)
            }
          }}
        >
          {mode === 'export' || mode === 'exportAll' ? 'Close' : 'Import'}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
