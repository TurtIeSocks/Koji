import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'

import TileServerForm from './TileServerForm'

export default function TileServerCreate() {
  const notify = useNotify()
  const redirect = useRedirect()

  const onSuccess = () => {
    notify('TileServer created successfully')
    redirect('list', 'tileserver')
  }

  return (
    <Create title="Add a TileServer" mutationOptions={{ onSuccess }}>
      <SimpleForm>
        <TileServerForm />
      </SimpleForm>
    </Create>
  )
}
