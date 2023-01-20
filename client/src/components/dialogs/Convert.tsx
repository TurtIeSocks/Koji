import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import {
  Box,
  FormControl,
  FormControlLabel,
  Switch,
  Typography,
} from '@mui/material'
import { GeoJSON } from 'react-leaflet'
import type { FeatureCollection } from 'geojson'

import { ToConvert } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { Code } from '@components/Code'
import MultiOptions from '@components/drawer/inputs/MultiOptions'
import { safeParse } from '@services/utils'
import { convert } from '@services/fetches'
import Map from '@components/Map'

import BaseDialog from './Base'

export default function ConvertDialog() {
  const open = useStatic((s) => s.dialogs.convert)
  const polygonExportMode = usePersist((s) => s.polygonExportMode)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const [code, setCode] = React.useState('')
  const [converted, setConverted] = React.useState('')
  const [previewGeojson, setPreviewGeojson] = React.useState<FeatureCollection>(
    {
      type: 'FeatureCollection',
      features: [],
    },
  )

  const [showPreview, setShowPreview] = React.useState(false)

  const convertCode = async (incoming: ToConvert) => {
    await convert(incoming, polygonExportMode, simplifyPolygons)
      .then((res) => {
        if (typeof res === 'string') {
          setConverted(res)
        } else {
          setConverted(JSON.stringify(res, null, 2))
        }
        return res
      })
      .then((res) =>
        convert<FeatureCollection>(res, 'featureCollection', false).then(
          (res2) => setPreviewGeojson(res2),
        ),
      )
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
            minWidth="40vw"
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
        <Grid2 xs={12} sm={6} container textAlign="left">
          <Grid2 xs={6}>
            <Typography variant="h3" align="center" my={1}>
              Result
            </Typography>
          </Grid2>
          <Grid2 xs={6}>
            <FormControl>
              <FormControlLabel
                value={simplifyPolygons}
                label="Preview"
                control={
                  <Switch
                    value={simplifyPolygons}
                    onChange={() => setShowPreview((prev) => !prev)}
                  />
                }
              />
            </FormControl>
          </Grid2>

          {showPreview ? (
            <Map
              forcedLocation={[
                ((previewGeojson.bbox?.[1] || 0) +
                  (previewGeojson.bbox?.[3] || 0)) /
                  2,
                ((previewGeojson.bbox?.[0] || 0) +
                  (previewGeojson.bbox?.[2] || 0)) /
                  2,
              ]}
              style={{ minWidth: '40vw', minHeight: '80vh' }}
            >
              <GeoJSON data={previewGeojson} />
            </Map>
          ) : (
            <Code minWidth="40vw">{converted}</Code>
          )}
        </Grid2>
      </Grid2>
    </BaseDialog>
  )
}
