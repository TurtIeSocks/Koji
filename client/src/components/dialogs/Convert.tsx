import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import {
  Box,
  Divider,
  FormControl,
  FormControlLabel,
  Switch,
  Typography,
} from '@mui/material'

import { CONVERSION_TYPES, GEOMETRY_CONVERSION_TYPES } from '@assets/constants'
import { Conversions, FeatureCollection } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { Code } from '@components/Code'
import MultiOptions from '@components/drawer/inputs/MultiOptions'
import { safeParse } from '@services/utils'
import { convert } from '@services/fetches'
import Map from '@components/Map'
import GeoJsonWrapper from '@components/GeojsonWrapper'

import BaseDialog from './Base'

export default function ConvertDialog({
  fullScreen = false,
}: {
  fullScreen?: boolean
}) {
  const open = useStatic((s) => s.dialogs.convert)
  const polygonExportMode = usePersist((s) => s.polygonExportMode)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)
  const geometryType = usePersist((s) => s.geometryType)

  const containerRef = React.useRef<HTMLDivElement>(null)

  const [code, setCode] = React.useState('')
  const [converted, setConverted] = React.useState('')
  const [previewGeojson, setPreviewGeojson] = React.useState<FeatureCollection>(
    {
      type: 'FeatureCollection',
      features: [],
    },
  )

  const [showPreview, setShowPreview] = React.useState(false)

  const convertCode = async (incoming: Conversions) => {
    await convert(incoming, polygonExportMode, simplifyPolygons, geometryType)
      .then((res) => {
        if (typeof res === 'string') {
          setConverted(res)
        } else {
          setConverted(JSON.stringify(res, null, 2))
        }
        return res
      })
      .then((res) =>
        convert<FeatureCollection>(
          res,
          'featureCollection',
          false,
          geometryType,
        ).then((res2) => setPreviewGeojson(res2)),
      )
  }

  const reset = () => {
    setCode('')
    setConverted('')
    setPreviewGeojson({
      type: 'FeatureCollection',
      features: [],
    })
  }

  React.useEffect(() => {
    reset()
  }, [open])

  React.useEffect(() => {
    if (code) {
      const parsed = safeParse<Conversions>(code)
      if (code.startsWith('{') || code.startsWith('[') ? !parsed.error : code) {
        convertCode(parsed.error ? code : parsed.value)
      } else {
        setConverted(parsed.error.toString())
      }
    }
  }, [polygonExportMode, simplifyPolygons, geometryType])

  const height = containerRef.current?.clientHeight.toString() ?? 0

  return (
    <BaseDialog
      open={fullScreen || open}
      onClose={
        fullScreen
          ? undefined
          : () =>
              useStatic.setState((prev) => ({
                dialogs: { ...prev.dialogs, convert: false },
              }))
      }
      title="Polygon Conversion Playground"
      Components={{
        Dialog: { maxWidth: 'xl', fullScreen },
        DialogContent: { ref: containerRef, sx: { pb: 0, height: '70vh' } },
        DialogActions: {
          children: (
            <>
              <MultiOptions
                field="polygonExportMode"
                buttons={CONVERSION_TYPES}
                type="select"
                label="Select Format"
              />
              <MultiOptions
                field="geometryType"
                buttons={GEOMETRY_CONVERSION_TYPES}
                type="select"
                label="Select Geometry Type"
              />
              <Divider
                flexItem
                orientation="vertical"
                sx={{ width: 2, ml: 2, mr: 0.5, color: 'white' }}
              />
              <FormControl
                size="small"
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Switch
                  onChange={(_e, v) =>
                    usePersist.setState({ simplifyPolygons: v })
                  }
                  checked={!!simplifyPolygons}
                />
                <Typography variant="caption">Simplify Polygons</Typography>
              </FormControl>
              <Box sx={{ flex: '1 1 auto' }} />
            </>
          ),
        },
      }}
    >
      <Grid2 container minWidth="85vw">
        <Grid2 xs={12} sm={6} textAlign="left">
          <Typography variant="h3" align="center" my={1}>
            Input
          </Typography>
          <Code
            minWidth="40vw"
            maxWidth="98%"
            height={height ? `${+height - 70}px` : '75vh'}
            code={code}
            setCode={async (newCode) => {
              if (!newCode) {
                reset()
              }
              setCode(newCode)
              const parsed = safeParse<Conversions>(newCode)
              if (
                newCode.startsWith('{') || newCode.startsWith('[')
                  ? !parsed.error
                  : newCode
              ) {
                await convertCode(parsed.error ? newCode : parsed.value)
              } else {
                setConverted(parsed.error.toString())
              }
            }}
          />
        </Grid2>
        <Grid2 xs={12} sm={6} container>
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
          <Grid2 xs={12} textAlign="left">
            {showPreview ? (
              <Map
                forcedLocation={
                  previewGeojson.bbox?.every((x) => typeof x === 'number')
                    ? [
                        ((previewGeojson.bbox?.[1] || 0) +
                          (previewGeojson.bbox?.[3] || 0)) /
                          2,
                        ((previewGeojson.bbox?.[0] || 0) +
                          (previewGeojson.bbox?.[2] || 0)) /
                          2,
                      ]
                    : usePersist.getState().location
                }
                style={{
                  minWidth: '40vw',
                  maxWidth: '98%',
                  height: height ? `${+height - 70}px` : '75vh',
                }}
              >
                <GeoJsonWrapper
                  key={JSON.stringify(previewGeojson)}
                  data={previewGeojson}
                />
              </Map>
            ) : (
              <Code
                minWidth="40vw"
                maxWidth="98%"
                height={height ? `${+height - 70}px` : '75vh'}
              >
                {converted}
              </Code>
            )}
          </Grid2>
        </Grid2>
      </Grid2>
    </BaseDialog>
  )
}
