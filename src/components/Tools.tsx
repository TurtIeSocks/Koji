import React from 'react'
import { useQuery } from '@apollo/client'

import { instances } from '@services/queries'

export default function Tools() {
  const { data } = useQuery(instances)

  return (
    <div />
  )
}