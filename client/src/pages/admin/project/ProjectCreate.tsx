import * as React from 'react'
import { Create, useNotify, useRedirect } from 'react-admin'

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
      <ProjectForm />
    </Create>
  )
}
