import * as React from 'react'
import { useStore } from '@hooks/useStore'
import type {
  Feature,
  LineString,
  MultiPoint,
  MultiLineString,
  Point,
  Polygon,
  MultiPolygon,
} from 'geojson'

import useSyncGeojson from '@hooks/useSyncGeojson'

import KojiPoint from './Point'
import KojiLineString from './LineString'
import KojiPolygon from './Polygon'
import KojiMultiPoint from './MultiPoint'
import KojiMultiLineString from './MultiLineString'

export default function Vectors() {
  const geojson = useSyncGeojson()
  const radius = useStore((s) => s.radius)

  return (
    <>
      {geojson.features.map(
        (feature) =>
          ({
            Point: (
              <KojiPoint
                key={`point_${feature.id}`}
                feature={feature as Feature<Point>}
                radius={radius || 10}
              />
            ),
            MultiPoint: (
              <KojiMultiPoint
                key={`multiPoint_${feature.id}`}
                feature={feature as Feature<MultiPoint>}
                radius={radius || 10}
              />
            ),
            LineString: (
              <KojiLineString
                key={`line_${feature.id}`}
                feature={feature as Feature<LineString>}
              />
            ),
            MultiLineString: (
              <KojiMultiLineString
                key={`multiline_${feature.id}`}
                feature={feature as Feature<MultiLineString>}
              />
            ),
            Polygon: (
              <KojiPolygon
                key={`polygon_${feature.id}`}
                feature={feature as Feature<Polygon>}
              />
            ),
            MultiPolygon: (
              <KojiPolygon
                key={`polygon_${feature.id}`}
                feature={feature as Feature<MultiPolygon>}
              />
            ),
            GeometryCollection: null,
          }[feature.geometry.type] || null),
      )}
    </>
  )
}
