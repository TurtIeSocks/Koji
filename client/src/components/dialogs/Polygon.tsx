import * as React from 'react'
import {
  Dialog,
  DialogContent,
  DialogActions,
  Button,
  TextField,
} from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import useDeepCompareEffect from 'use-deep-compare-effect'
import distance from '@turf/distance'

import { CONVERSION_TYPES } from '@assets/constants'
import type { Conversions, Feature, FeatureCollection } from '@assets/types'
import MultiOptions from '@components/drawer/inputs/MultiOptions'
import SplitMultiPolygonsBtn from '@components/buttons/SplitMultiPolygons'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'
import { convert } from '@services/fetches'

import DialogHeader from './Header'
import { Code } from '../Code'

interface Props {
  open: string
  setOpen: (open: string) => void
  route?: boolean
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
  route = false,
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
  const [stats, setStats] = React.useState<{
    max: number
    total: number
    count: number
  }>({ max: 0, total: 0, count: 0 })

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
          const parsed: Conversions =
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
                id: f.id ?? `${f.properties?.__name}${f.properties?.__mode}`,
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

  useDeepCompareEffect(() => {
    if (route && open) {
      let max = 0
      let total = 0
      let count = 0
      if (feature.type === 'Feature') {
        if (feature.geometry.type === 'MultiPoint') {
          const { coordinates } = feature.geometry
          coordinates.forEach((point, j) => {
            const next = j ? coordinates[j + 1] : coordinates.at(-1)
            if (next) {
              const dis = distance(point, next, { units: 'meters' })
              if (dis > max) max = dis
              total += dis
            }
            count++
          })
        }
      } else {
        feature.features.forEach((f) => {
          if (f.geometry.type === 'MultiPoint') {
            const { coordinates } = f.geometry
            coordinates.forEach((point, j) => {
              const next = j ? coordinates[j + 1] : coordinates.at(-1)
              if (next) {
                const dis = distance(point, next, { units: 'meters' })
                if (dis > max) max = dis
                total += dis
              }
              count++
            })
          }
        })
      }
      setCode(JSON.stringify(feature, null, 2))
      setStats({ max, total, count })
    } else if (!route) {
      setStats({ max: 0, total: 0, count: 0 })
    }
  }, [route, open, feature])

  return (
    <Dialog
      open={open === (route ? 'route' : 'polygon')}
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
          ? `Export ${route ? 'Route' : 'Polygon'}`
          : `Import ${route ? 'Route' : 'Polygon'}`}
      </DialogHeader>
      <DialogContent
        sx={{ width: '90vw', minHeight: '60vh', overflow: 'auto' }}
      >
        <Grid2 container>
          <Grid2 xs={route ? 9 : 12} textAlign="left">
            <Code
              code={code}
              setCode={setCode}
              maxHeight="70vh"
              textMode={
                mode === 'export' || mode === 'exportAll'
                  ? polygonExportMode === 'text' ||
                    polygonExportMode === 'altText'
                  : !code.startsWith('{') && !code.startsWith('[')
              }
            />
          </Grid2>
          {route && (
            <Grid2 xs={3} container justifyContent="flex-start">
              <Grid2 xs={12}>
                <TextField
                  value={stats.count}
                  label="Count"
                  sx={{ width: '90%', my: 2 }}
                  disabled
                />
              </Grid2>
              <Grid2 xs={12}>
                <TextField
                  value={stats.max?.toFixed(2) || 0}
                  label="Max"
                  sx={{ width: '90%', my: 2 }}
                  InputProps={{ endAdornment: 'm' }}
                  disabled
                />
              </Grid2>
              <Grid2 xs={12}>
                <TextField
                  value={(stats.total / (stats.count || 1))?.toFixed(2) || 0}
                  label="Average"
                  sx={{ width: '90%', my: 2 }}
                  InputProps={{ endAdornment: 'm' }}
                  disabled
                />
              </Grid2>
              <Grid2 xs={12}>
                <TextField
                  value={stats.total?.toFixed(2) || 0}
                  label="Total"
                  sx={{ width: '90%', my: 2 }}
                  InputProps={{ endAdornment: 'm' }}
                  disabled
                />
              </Grid2>
            </Grid2>
          )}
        </Grid2>
      </DialogContent>
      <DialogActions>
        {(mode === 'export' || mode === 'exportAll') && (
          <>
            <MultiOptions
              field="polygonExportMode"
              buttons={CONVERSION_TYPES}
              type="select"
            />
            <SplitMultiPolygonsBtn
              fc={
                feature.type === 'FeatureCollection'
                  ? feature
                  : { type: 'FeatureCollection', features: [feature] }
              }
              setter={(newFc) => setCode(JSON.stringify(newFc, null, 2))}
            />
          </>
        )}
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
