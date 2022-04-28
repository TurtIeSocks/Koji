import React from 'react'
import { pokestop, gym, spawnpoint } from '@prisma/client'

export function Poi<T>({ poi }: { poi: T extends pokestop ? pokestop : gym}) {
  const { name, lat, lon, updated } = poi

  return (
    <div>
      <h6>{name}</h6>
      <p>{updated}</p>
      <p>{`${lat.toFixed(6)}, ${lon.toFixed(6)}`}</p>
    </div>
  )
}

export function Spawnpoint({ point }: { point: spawnpoint }) {
  const { despawn_sec, lat, lon, updated } = point
  const despawn = despawn_sec || 0
  const minute = despawn > 60 ? Math.round(despawn / 60) : despawn
  const minuteFixed = minute < 10 ? `0${minute}` : minute

  return (
    <div>
      <h6>{despawn ? `00:${minuteFixed}` : '?'}</h6>
      <p>{updated}</p>
      <p>{`${lat.toFixed(6)}, ${lon.toFixed(6)}`}</p>
    </div>
  )
}
