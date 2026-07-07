import { useWatch } from "react-hook-form";
import {
  TextInput,
  BooleanInput,
  SelectInput,
  ReferenceInput,
  AutocompleteInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";
import { WEBHOOK_MODES, WEBHOOK_METHODS, WEBHOOK_TOPICS } from "@/lib/constants";
import { HeadersInput } from "./headers-input";

export const WebhookFormFields = () => {
  const mode = useWatch({ name: "mode" }) ?? "event";
  return (
    <>
      <TextInput source="name" validate={required()} />
      <TextInput source="url" validate={required()} />
      <SelectInput source="mode" choices={[...WEBHOOK_MODES]} defaultValue="event" validate={required()} />
      <BooleanInput source="active" defaultValue={true} />
      <ReferenceInput source="project_id" reference="project">
        <AutocompleteInput
          label="Project"
          helperText="Leave empty for a global subscription"
        />
      </ReferenceInput>
      {mode === "ping" && (
        <SelectInput source="method" choices={[...WEBHOOK_METHODS]} defaultValue="GET" />
      )}
      {mode === "event" && (
        <>
          <TextInput source="secret" label="Secret (HMAC key)" />
          <AutocompleteArrayInput
            source="topics"
            choices={[...WEBHOOK_TOPICS]}
            helperText="Empty = all topics"
          />
        </>
      )}
      <HeadersInput />
    </>
  );
};
