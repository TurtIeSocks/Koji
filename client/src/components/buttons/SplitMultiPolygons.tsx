import Button, { ButtonProps } from '@mui/material/Button/Button'
import { splitMultiPolygons } from '@services/utils'
import type { FeatureCollection } from 'geojson'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: FeatureCollection
  setter: (fc: FeatureCollection) => void
}

export default function SplitMultiPolygonsBtn({ fc, setter, ...rest }: Props) {
  return (
    <Button
      onClick={() => {
        setter(splitMultiPolygons(fc))
      }}
      {...rest}
    >
      Split Multi Polygons
    </Button>
  )
}
