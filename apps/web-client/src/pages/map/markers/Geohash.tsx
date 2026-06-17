import * as React from 'react'
import geohash from 'ngeohash'
import { Circle } from 'react-leaflet'
import { usePersist } from '@hooks/usePersist'

import StyledPopup from '../popups/Styled'

export function GeohashMarker({ hash }: { hash: string }) {
  const x = geohash.decode(hash)
  const radius = usePersist((s) => s.radius)
  return (
    <>
      <Circle
        center={[x.latitude, x.longitude]}
        radius={radius || 10}
        fillOpacity={0.25}
        opacity={0.25}
        color="black"
        pane="dev_markers"
        pmIgnore
        snapIgnore
      >
        <StyledPopup>
          <div>Hash: {hash}</div>
        </StyledPopup>
      </Circle>
      <Circle
        center={[x.latitude, x.longitude]}
        radius={1}
        pathOptions={{
          fillColor: 'black',
          color: 'black',
        }}
        pane="dev_markers"
        pmIgnore
        snapIgnore
      />
    </>
  )
}
