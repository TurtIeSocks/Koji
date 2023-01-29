import * as React from 'react'
import shallow from 'zustand/shallow'

import { useShapes } from '@hooks/useShapes'
import { usePersist } from '@hooks/usePersist'
import { useDbCache } from '@hooks/useDbCache'

import { MemoPoint } from './Point'
import { KojiLineString } from './LineString'
import { MemoPolygon } from './Polygon'
import { MemoMultiPoint } from './MultiPoint'
import { MemoMultiLineString } from './MultiLineString'

export function Points() {
  const shapes = useShapes((s) => s.Point)
  const radius = usePersist((s) => s.radius)
  const setActiveMode = usePersist((s) => s.setActiveMode)
  const { getFromKojiKey } = useDbCache.getState()

  return (
    <React.Fragment key={setActiveMode}>
      {Object.entries(shapes).map(([id, feature]) => (
        <MemoPoint
          key={id}
          feature={feature}
          radius={radius || 10}
          dbRef={getFromKojiKey(feature.properties.__multipoint_id)}
        />
      ))}
    </React.Fragment>
  )
}

export function MultiPoints() {
  const shapes = useShapes((s) => s.MultiPoint)
  const radius = usePersist((s) => s.radius)
  const setActiveMode = usePersist((s) => s.setActiveMode)
  const { getFromKojiKey } = useDbCache.getState()

  return (
    <React.Fragment key={setActiveMode}>
      {Object.entries(shapes).map(([id, feature]) => (
        <MemoMultiPoint
          key={id}
          feature={feature}
          radius={radius || 10}
          dbRef={getFromKojiKey(id)}
        />
      ))}
    </React.Fragment>
  )
}

export function LineStrings() {
  const shapes = useShapes((s) => s.LineString)

  return (
    <>
      {Object.entries(shapes).map(([id, feature]) => (
        <KojiLineString key={id} feature={feature} />
      ))}
    </>
  )
}

export function MultiLineStrings() {
  const shapes = useShapes((s) => s.MultiLineString)

  return (
    <>
      {Object.entries(shapes).map(([id, feature]) => (
        <MemoMultiLineString key={id} feature={feature} />
      ))}
    </>
  )
}

export function Polygons() {
  const shapes = useShapes(
    (s) => ({ ...s.Polygon, ...s.MultiPolygon }),
    shallow,
  )
  const { getFromKojiKey } = useDbCache.getState()

  return (
    <>
      {Object.entries(shapes).map(([id, feature]) => (
        <MemoPolygon key={id} feature={feature} dbRef={getFromKojiKey(id)} />
      ))}
    </>
  )
}

// interface Props {
//   id: Feature['id']
//   feature: Feature<GeometryCollection>
//   radius?: number
// }

// export function GeometryFeature({ id, feature, radius }: Props) {
//   return (
//     <>
//       {feature.geometry.geometries.map((geometry, i) => {
//         switch (geometry.type) {
//           case 'Point':
//             return (
//               <MemoPoint
//                 key={`${id}_${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//                 radius={radius || 10}
//               />
//             )
//           case 'MultiPoint':
//             return (
//               <KojiMultiPoint
//                 key={`${id}_${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//                 radius={radius || 10}
//               />
//             )
//           case 'LineString':
//             return (
//               <KojiLineString
//                 key={`${id}__${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//               />
//             )
//           case 'MultiLineString':
//             return (
//               <MemoMultiLineString
//                 key={`${id}_${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//               />
//             )
//           case 'Polygon':
//             return (
//               <MemoPolygon
//                 key={`${id}_${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//               />
//             )
//           case 'MultiPolygon':
//             return (
//               <MemoPolygon
//                 key={`${id}_${feature.id}`}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//               />
//             )
//           case 'GeometryCollection':
//             return (
//               <GeometryFeature
//                 key={`${id}_${feature.id}`}
//                 id={id}
//                 feature={{
//                   ...feature,
//                   id: `${feature.id}_${i}`,
//                   geometry,
//                 }}
//                 radius={radius || 10}
//               />
//             )
//           default:
//             return null
//         }
//       })}
//     </>
//   )
// }

// export function GeometryCollections() {
//   const shapes = useShapes((s) => s.GeometryCollection)
//   const radius = usePersist((s) => s.radius)

//   return (
//     <>
//       {Object.entries(shapes).map(([id, feature]) => (
//         <GeometryFeature
//           key={id}
//           id={id}
//           feature={feature}
//           radius={radius || 10}
//         />
//       ))}
//     </>
//   )
// }
