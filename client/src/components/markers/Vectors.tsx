import * as React from 'react'
import { useStore } from '@hooks/useStore'
import type {
  Feature,
  LineString,
  MultiPoint as MPType,
  MultiLineString,
  Point,
  Polygon as PGType,
  MultiPolygon,
} from 'geojson'

import useSyncGeojson from '@hooks/useSyncGeojson'

import Circle from './Circle'
import Polyline from './Polyline'
import Polygon from './Polygon'
import MultiPoint from './MultiPoint'

export default function Vectors() {
  const geojson = useSyncGeojson()
  const radius = useStore((s) => s.radius)

  return (
    <>
      {geojson.features.map(
        (each) =>
          ({
            Point: (
              <Circle
                key={`point_${each.id}`}
                feature={each as Feature<Point>}
                radius={radius || 10}
              />
            ),
            MultiPoint: (
              <MultiPoint
                key={`multiPoint_${each.id}`}
                feature={each as Feature<MPType>}
                radius={radius || 10}
              />
            ),
            LineString: (
              <Polyline
                key={`line_${each.id}`}
                feature={each as Feature<LineString>}
              />
            ),
            MultiLineString: (
              each as Feature<MultiLineString>
            ).geometry.coordinates.map((coords) => (
              <Polyline
                key={`multiLine_${coords}`}
                feature={{
                  ...each,
                  geometry: { type: 'LineString', coordinates: coords },
                }}
              />
            )),
            Polygon: (
              <Polygon
                key={`polygon_${each.id}`}
                feature={each as Feature<PGType>}
              />
            ),
            MultiPolygon: (
              each as Feature<MultiPolygon>
            ).geometry.coordinates.map((coords) => (
              <Polygon
                key={`multiPolygon_${coords}`}
                feature={{
                  ...each,
                  geometry: { type: 'Polygon', coordinates: coords },
                }}
              />
            )),
            GeometryCollection: null,
          }[each.geometry.type] || null),
      )}
    </>
    //   {Object.values(points).map((each) => (
    //     <Circle key={`point_${each.id}`} feature={each} radius={radius || 10} />
    //   ))}
    //   {Object.values(lineStrings).map((each) => (
    //     <Polyline key={`line_${each.id}`} feature={each} />
    //   ))}
    //   {Object.values(polygons).map((each) => (
    //     <Polygon key={`polygon_${each.id}`} feature={each} />
    //   ))}
    //   {Object.values(multiPoints).map((each) => (
    // <MultiPoint
    //   key={`multiPoint_${each.id}`}
    //   feature={each}
    //   radius={radius || 10}
    // />
    //   ))}
    //   {Object.values(multiLineStrings).map((each) =>
    // each.geometry.coordinates.map((coords) => (
    //   <Polyline
    //     key={`multiLine_${coords}`}
    //     feature={{
    //       ...each,
    //       geometry: { type: 'LineString', coordinates: coords },
    //     }}
    //   />
    // )),
    //   )}
    //   {Object.values(multiPolygons).map((each) =>
    // each.geometry.coordinates.map((coords) => (
    //   <Polygon
    //     key={`multiPolygon_${coords}`}
    //     feature={{
    //       ...each,
    //       geometry: { type: 'Polygon', coordinates: coords },
    //     }}
    //   />
    // )),
    //   )}
    // </>
  )
}
