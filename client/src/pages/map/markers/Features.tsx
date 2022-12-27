/* eslint-disable react/jsx-no-useless-fragment */
import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import type {
  Feature,
  LineString,
  MultiPoint,
  MultiLineString,
  Point,
  Polygon,
  MultiPolygon,
  Geometry,
} from 'geojson'

import useSyncGeojson from '@hooks/useSyncGeojson'

import KojiPoint from './Point'
import KojiLineString from './LineString'
import KojiPolygon from './Polygon'
import KojiMultiPoint from './MultiPoint'
import KojiMultiLineString from './MultiLineString'

export function Geometries({
  feature,
  radius,
}: {
  feature: Feature
  radius: number
}) {
  return (
    <>
      {{
        Point: (
          <KojiPoint
            key={`point_${feature.id}_${
              (feature as Feature<Point>).geometry.coordinates.length
            }`}
            feature={feature as Feature<Point>}
            radius={radius || 10}
          />
        ),
        MultiPoint: (
          <KojiMultiPoint
            key={`multiPoint_${feature.id}_${
              (feature as Feature<MultiPoint>).geometry.coordinates.length
            }`}
            feature={feature as Feature<MultiPoint>}
            radius={radius || 10}
          />
        ),
        LineString: (
          <KojiLineString
            key={`line_${feature.id}_${
              (feature as Feature<LineString>).geometry.coordinates.length
            }`}
            feature={feature as Feature<LineString>}
          />
        ),
        MultiLineString: (
          <KojiMultiLineString
            key={`multiline_${feature.id}_${
              (feature as Feature<MultiLineString>).geometry.coordinates.length
            }`}
            feature={feature as Feature<MultiLineString>}
          />
        ),
        Polygon: (
          <KojiPolygon
            key={`polygon_${feature.id}_${
              (feature as Feature<Polygon>).geometry.coordinates.length
            }`}
            feature={feature as Feature<Polygon>}
          />
        ),
        MultiPolygon: (
          <KojiPolygon
            key={`polygon_${feature.id}_${
              (feature as Feature<MultiPolygon>).geometry.coordinates.length
            }`}
            feature={feature as Feature<MultiPolygon>}
          />
        ),
        GeometryCollection:
          feature.geometry.type === 'GeometryCollection'
            ? feature.geometry.geometries.map((geometry, i) => (
                <Geometries
                  key={`geometry_${feature.id}`}
                  feature={
                    {
                      ...feature,
                      id: `${feature.id}_${i}`,
                      geometry,
                    } as Feature<Geometry>
                  }
                  radius={radius || 10}
                />
              ))
            : null,
      }[feature.geometry.type] || null}
    </>
  )
}

export default function Features() {
  const geojson = useSyncGeojson()
  const radius = usePersist((s) => s.radius)

  return (
    <>
      {geojson.features.map((feature) => (
        <Geometries key={feature.id} feature={feature} radius={radius || 10} />
      ))}
    </>
  )
}
