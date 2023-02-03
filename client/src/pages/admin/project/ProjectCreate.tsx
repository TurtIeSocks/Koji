import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'

import ProjectForm from './ProjectForm'

export default function ProjectCreate() {
  const notify = useNotify()
  const redirect = useRedirect()

  const onSuccess = () => {
    notify('Project created successfully')
    redirect('list', 'project')
  }

  return (
    <Create title="Create a Project" mutationOptions={{ onSuccess }}>
      <SimpleForm>
        <ProjectForm />
      </SimpleForm>
    </Create>
  )
}
