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
  const dragonite = mode === "dragonite";
  return (
    <>
      <TextInput source="name" validate={required()} />
      <TextInput
        source="url"
        label={dragonite ? "Dragonite base URL" : undefined}
        validate={required()}
      />
      <SelectInput source="mode" choices={[...WEBHOOK_MODES]} defaultValue="event" validate={required()} />
      <BooleanInput source="active" defaultValue={true} />
      <ReferenceInput source="project_id" reference="project">
        <AutocompleteInput
          label="Project"
          helperText={
            dragonite
              ? "Leave empty to reach every geofence"
              : "Leave empty for a global subscription"
          }
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
      {dragonite && (
        <TextInput
          source="secret"
          label="Bearer"
          helperText="Sent to Dragonite as its bearer token"
          validate={required()}
        />
      )}
      {!dragonite && <HeadersInput />}
    </>
  );
};
