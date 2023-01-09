import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import Typography from '@mui/material/Typography'
import type { FeatureCollection } from 'geojson'

import JsonFile from '@components/drawer/manage/Json'
import ShapeFile from '@components/drawer/manage/ShapeFile'
import { Divider } from '@mui/material'
import InstanceSelect from '@components/drawer/manage/Instance'
import { useStatic } from '@hooks/useStatic'

const ImportStep = React.forwardRef<
  HTMLDivElement,
  {
    handleChange: (
      geojson: FeatureCollection,
      updatedSelected?: boolean,
    ) => void
  }
>(({ handleChange }, ref) => {
  const { scannerSelected } = useStatic((s) => s.importWizard)

  return (
    <Grid2 container ref={ref}>
      {/* JSON */}
      <Grid2 xs={2}>
        <Typography variant="h5">JSON</Typography>
      </Grid2>
      <Grid2 xs={6}>
        <Typography sx={{ my: 1 }}>
          Upload a JSON file, such as an <code>areas.json</code> from ReactMap,
          a <code>geofence.json</code> from Poracle, or any GeoJSON.
        </Typography>
      </Grid2>
      <Grid2 xs={4}>
        <JsonFile setter={handleChange} />
      </Grid2>
      <Divider sx={{ width: '95%', my: 1 }} />
      {/* Shapefile */}
      <Grid2 xs={2}>
        <Typography variant="h5">Shapefile</Typography>
      </Grid2>
      <Grid2 xs={6}>
        <Typography sx={{ my: 1 }}>
          Accepts either a <code>.shp</code> file or a combination of{' '}
          <code>.shp</code> and <code>.dbf</code> files to add extra
          information.
        </Typography>
      </Grid2>
      <Grid2 xs={4}>
        <ShapeFile setter={handleChange} />
      </Grid2>
      <Divider sx={{ width: '95%', my: 1 }} />
      {/* Scanner */}
      <Grid2 xs={2}>
        <Typography variant="h5">Scanner</Typography>
      </Grid2>
      <Grid2 xs={6}>
        <Typography sx={{ my: 1 }}>
          Import fences directly from your scanner database.
        </Typography>
      </Grid2>
      <Grid2 xs={4}>
        <InstanceSelect
          endpoint="/internal/routes/from_scanner"
          setGeojson={(geo) => handleChange(geo, true)}
          filters={['AutoTth', 'AutoQuest', 'PokemonIv', 'AutoPokemon']}
          initialState={[...scannerSelected]}
        />
      </Grid2>
      <Divider sx={{ width: '95%', my: 1 }} />
      {/* Nominatim */}
      <Grid2 xs={12}>
        <Typography sx={{ my: 1 }}>
          Nominatim and OSM searching coming soon
        </Typography>
      </Grid2>
    </Grid2>
  )
})

ImportStep.displayName = 'ImportStep'

export default ImportStep
