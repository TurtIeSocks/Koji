import * as React from 'react'
import { Dialog, DialogContent, DialogActions, Button } from '@mui/material'

import type { Feature, FeatureCollection } from 'geojson'
import { featureCollection } from '@turf/helpers'

import type { ArrayInput, ObjectInput } from '@assets/types'
import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'
import { arrayToFeature, geojsonToExport, textToFeature } from '@services/utils'

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
    switch (polygonExportMode) {
      case 'geojson':
        if (mode === 'export') {
          setCode(JSON.stringify(feature, null, 2))
        }
        break
      case 'array':
        if (mode === 'export') {
          const array = geojsonToExport(feature)
          setCode(
            JSON.stringify(
              feature.geometry.type === 'Polygon' ? array[0] : array,
              null,
              2,
            ),
          )
        }
        break
      case 'object':
        if (mode === 'export') {
          const object = geojsonToExport(feature, true)
          setCode(
            JSON.stringify(
              feature.geometry.type === 'Polygon' ? object[0] : object,
              null,
              2,
            ),
          )
        }
        break
      case 'text':
        if (mode === 'export') {
          const array = geojsonToExport(feature)
          setCode(array.flatMap((a) => a).join('\n'))
        }
        break
      default:
        break
    }
  }, [polygonExportMode, code])

  React.useEffect(() => {
    if (mode === 'import' && code) {
      try {
        const parsed:
          | string
          | ObjectInput
          | ArrayInput
          | Feature
          | FeatureCollection =
          code.startsWith('{') || code.startsWith('[') ? JSON.parse(code) : code
        const geojson: FeatureCollection = (() => {
          if (typeof parsed === 'string') {
            return featureCollection([textToFeature(parsed.trim())])
          }
          if (Array.isArray(parsed)) {
            return featureCollection([arrayToFeature(parsed)])
          }
          if (parsed.type === 'Feature') {
            return featureCollection([parsed])
          }
          if (parsed.type === 'FeatureCollection') {
            return parsed
          }
          return { type: 'FeatureCollection', features: [] }
        })()
        if (geojson.type === 'FeatureCollection') {
          setTempGeojson(geojson)
        }
        setError('')
      } catch (e) {
        if (e instanceof Error) {
          setError(e.message)
        }
      }
    }
  }, [code])
  return (
    <Dialog
      open={!!open}
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
