import {
  DataTable,
  BooleanField,
  ReferenceField,
  TextField,
  FilterLiveSearch,
  ReferenceInput,
  AutocompleteInput,
} from "@/components/admin";
import { FilterLiveForm } from "ra-core";
import { ListLive } from "@/components/realtime";
import type { RaRecord } from "ra-core";

const renderProject = (record: RaRecord) =>
  record?.project_id == null ? (
    <span>Global</span>
  ) : (
    <ReferenceField source="project_id" reference="project" record={record}>
      <TextField source="name" />
    </ReferenceField>
  );

export const WebhookList = () => (
  <ListLive
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
        <FilterLiveForm>
          <ReferenceInput source="project" reference="project">
            <AutocompleteInput label="Project" />
          </ReferenceInput>
        </FilterLiveForm>
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="url" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="project_id" label="Project" render={renderProject} />
      <DataTable.Col source="active" label="Active">
        <BooleanField source="active" />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
