import * as React from 'react'
import { Edit, SimpleForm } from 'react-admin'

import type { KojiRoute } from '@assets/types'

import RouteForm from './RouteForm'

const transformPayload = async (route: KojiRoute) => {
  return {
    ...route,
    geometry:
      typeof route.geometry === 'string'
        ? JSON.parse(route.geometry)
        : route.geometry,
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
