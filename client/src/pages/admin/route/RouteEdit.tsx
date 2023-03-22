import * as React from 'react'
import { Edit, SimpleForm } from 'react-admin'

import type { KojiRoute } from '@assets/types'

import RouteForm from './RouteForm'

const transformPayload = async (route: KojiRoute) => {
  const { geometry: raw, ...rest } = route
  const geometry = typeof raw === 'string' ? JSON.parse(raw) : raw
  return {
    ...rest,
    geometry,
  }
}

export default function RouteEdit() {
  return (
    <Edit mutationMode="pessimistic" transform={transformPayload}>
      <SimpleForm>
        <RouteForm />
      </SimpleForm>
    </Edit>
  )
}
