import * as React from "react";
import type { Any } from "@/lib/any";
import { useState, useEffect, useRef } from "react";
import type { RaRecord } from "shadmin-core";
import {
  ListBase,
  getElementsFromRecords,
  InferredElement,
  useListContext,
  usePrevious,
  useResourceContext,
} from "shadmin-core";
import { useLocation } from "react-router";
import type { ListProps, ListViewProps } from "@/components/admin/list/list";
import { ListView } from "@/components/admin/list/list";
import { capitalize, singularize } from "inflection";
import { DataTable } from "@/components/admin/list/data-table";
import { ArrayField } from "@/components/admin/fields/array-field";
import { BadgeField } from "@/components/admin/fields/badge-field";
import { ReferenceField } from "@/components/admin/fields/reference-field";
import { SingleFieldList } from "@/components/admin/list/single-field-list";
import { ReferenceArrayField } from "@/components/admin/fields/reference-array-field";
import { GuesserEmpty } from "@/components/admin/guessers/guesser-empty";

/**
 * A list page that automatically generates a DataTable from your data.
 *
 * Inspects the first record to infer field types and automatically creates appropriate columns.
 * Useful for rapid prototyping. Logs generated code to console.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list#scaffolding-a-list-page ListGuesser documentation}
 *
 * @example
 * import { Admin, ListGuesser } from '@/components/admin';
 * import { Resource } from 'ra-core';
 * import { dataProvider } from './dataProvider';
 *
 * const App = () => (
 *   <Admin dataProvider={dataProvider}>
 *     // ...
 *     <Resource name="posts" list={ListGuesser} />
 *   </Admin>
 * );
 */
function ListGuesser<RecordType extends RaRecord = RaRecord>(
  props: Omit<ListProps, "children"> & { enableLog?: boolean },
) {
  const {
    debounce,
    disableAuthentication,
    disableSyncWithLocation,
    empty,
    exporter,
    filter,
    filterDefaultValues,
    perPage,
    resource,
    sort,
    ...rest
  } = props;
  // force a rerender of this component when any list parameter changes
  // otherwise the ListBase won't be rerendered when the sort changes
  // and the following check won't be performed
  useLocation();
  // keep previous data, unless the resource changes
  const resourceFromContext = useResourceContext(props);
  const previousResource = usePrevious(resourceFromContext);
  const keepPreviousData = previousResource === resourceFromContext;
  return (
    <ListBase<RecordType>
      debounce={debounce}
      disableAuthentication={disableAuthentication}
      disableSyncWithLocation={disableSyncWithLocation}
      empty={empty === undefined ? <GuesserEmpty /> : empty}
      exporter={exporter}
      filter={filter}
      filterDefaultValues={filterDefaultValues}
      perPage={perPage}
      resource={resource}
      queryOptions={{
        placeholderData: (previousData) =>
          keepPreviousData ? previousData : undefined,
      }}
      sort={sort}
    >
      <ListViewGuesser {...rest} />
    </ListBase>
  );
}

function ListViewGuesser(
  props: Omit<ListViewProps, "children"> & { enableLog?: boolean },
) {
  const { data } = useListContext();
  const resource = useResourceContext();
  const [child, setChild] = useState<React.ReactElement | null>(null);
  const hasInferredRef = useRef(false);
  const { enableLog = process.env.NODE_ENV === "development", ...rest } = props;

  // biome-ignore lint/correctness/useExhaustiveDependencies: reset effect is intentionally keyed on resource only — it clears the inferred view so the next effect re-derives it when the resource changes.
  useEffect(() => {
    hasInferredRef.current = false;
    setChild(null);
  }, [resource]);

  const hasData = (data?.length ?? 0) > 0;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the guesser derives the default columns once from the first non-empty data page (gated by hasInferredRef); intentionally not reactive to later data changes, which would regenerate the UI and re-spam the console log.
  useEffect(() => {
    if (hasInferredRef.current || !hasData) return;
    if (!resource) {
      throw new Error("Cannot use <ListGuesser> outside of a ResourceContext");
    }
    const inferredElements = getElementsFromRecords(data ?? [], listFieldTypes);
    const inferredChild = new InferredElement(
      listFieldTypes.table,
      null,
      inferredElements,
    );
    const inferredChildElement = inferredChild.getElement();
    const representation = inferredChild.getRepresentation();
    if (!inferredChildElement || !representation) {
      return;
    }
    hasInferredRef.current = true;
    setChild(inferredChildElement);

    if (!enableLog) return;
    const components = ["List"]
      .concat(
        Array.from(
          new Set(
            Array.from(representation.matchAll(/<([^/\s.>]+)/g))
              .map((match) => match[1])
              .filter((component) => component !== "span"),
          ),
        ),
      )
      .sort();
    console.log(
      `Guessed List:

${components
  .map(
    (component) =>
      `import { ${component} } from "@/components/admin/${kebabCase(
        component,
      )}";`,
  )
  .join("\n")}

export const ${capitalize(singularize(resource))}List = () => (
    <List>
${inferredChild.getRepresentation()}
    </List>
);`,
    );
  }, [hasData, resource, enableLog]);

  return <ListView {...rest}>{child}</ListView>;
}

const listFieldTypes = {
  table: {
    component: (props: Any) => {
      return <DataTable {...props} />;
    },
    representation: (
      _props: Any,
      children: { getRepresentation: () => string }[],
    ) =>
      `        <DataTable>
${children
  .map((child) => `            ${child.getRepresentation()}`)
  .join("\n")}
        </DataTable>`,
  },

  reference: {
    component: (props: Any) => (
      <DataTable.Col source={props.source}>
        <ReferenceField source={props.source} reference={props.reference} />
      </DataTable.Col>
    ),
    representation: (props: Any) =>
      `<DataTable.Col source="${props.source}">
                <ReferenceField source="${props.source}" reference="${props.reference}" />
            </DataTable.Col>`,
  },
  array: {
    component: ({ children, ...props }: Any) => {
      const childrenArray = React.Children.toArray(children);
      return (
        <DataTable.Col source={props.source}>
          <ArrayField source={props.source}>
            <SingleFieldList>
              <BadgeField
                source={
                  childrenArray.length > 0 &&
                  React.isValidElement(childrenArray[0]) &&
                  (childrenArray[0].props as Any).source
                }
              />
            </SingleFieldList>
          </ArrayField>
        </DataTable.Col>
      );
    },
    representation: (props: Any, children: Any) =>
      `<DataTable.Col source="${props.source}">
               <ArrayField source="${props.source}">
                    <SingleFieldList>
                        <BadgeField source="${
                          children.length > 0 && children[0].getProps().source
                        }" />
                   </SingleFieldList>
                </ArrayField>
            </DataTable.Col>`,
  },
  referenceArray: {
    component: (props: Any) => (
      <DataTable.Col {...props}>
        <ReferenceArrayField {...props} />
      </DataTable.Col>
    ),
    representation: (props: Any) =>
      `<DataTable.Col source="${props.source}">
                <ReferenceArrayField source="${props.source}" reference="${props.reference}" />
            </DataTable.Col>`,
  },
  string: {
    component: DataTable.Col,
    representation: (props: Any) =>
      `<DataTable.Col source="${props.source}" />`,
  },
};

const kebabCase = (name: string) => {
  return name
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1-$2")
    .toLowerCase();
};

export { ListGuesser };
