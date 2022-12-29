import * as React from 'react'
import type { PopupProps } from '@assets/types'

interface Props extends PopupProps {
  dis: number
}

export default function LineStringPopup({ dis }: Props) {
  return <>{dis.toFixed(2)}</>
}
