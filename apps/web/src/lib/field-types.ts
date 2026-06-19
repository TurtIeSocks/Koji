import type { BaseFieldProps } from "shadmin-core";
import type { ReactNode } from "react";
import type { UnknownRecord } from "./unknown-types";

export type SortOrder = "ASC" | "DESC";
export type TextAlign = "left" | "center" | "right" | "justify" | "inherit";

export interface FieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends Omit<BaseFieldProps<RecordType>, "resource"> {
  /**
   * A class name to apply to the root div element
   */
  className?: string;

  /**
   * A class name to apply to the cell element when used inside <DataTable>.
   */
  cellClassName?: string;

  /**
   * A class name to apply to the header cell element when used inside <DataTable>.
   */
  headerClassName?: string;

  /**
   * Label to use as column header when using <DataTable> or <SimpleShowLayout>.
   * Defaults to the capitalized field name. Set to false to disable the label.
   *
   * @see https://marmelab.com/react-admin/Fields.html#label
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="title" />
   *             <TextField source="body" label="Content" />
   *         </DataTable>
   *     </List>
   * );
   */
  label?: ReactNode | false;

  /**
   * Set it to false to disable the click handler on the column header when used inside <DataTable>.
   *
   * @see https://marmelab.com/react-admin/Fields.html#sortable
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="title" />
   *             <ReferenceField source="author_id" sortable={false}>
   *                 <TextField source="name" />
   *             </ReferenceField>
   *         </DataTable>
   *     </List>
   * );
   */
  sortable?: boolean;

  /**
   * The field to use for sorting when users click this column head, if sortable.
   *
   * @see https://marmelab.com/react-admin/Fields.html#sortby
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="title" />
   *             <ReferenceField source="author_id" sortBy="author.name">
   *                 <TextField source="name" />
   *             </ReferenceField>
   *         </DataTable>
   *     </List>
   * );
   */
  sortBy?: string;

  /**
   * The order used for sorting when users click this column head, if sortable.
   *
   * @see https://marmelab.com/react-admin/Fields.html#sortbyorder
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="title" />
   *             <DateField source="updated_at" sortByOrder="DESC" />
   *         </DataTable>
   *     </List>
   * );
   */
  sortByOrder?: SortOrder;

  /**
   * The text alignment for the cell content, when used inside <DataTable>.
   *
   * @see https://marmelab.com/react-admin/Fields.html#textalign
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="id" />
   *             <TextField source="title" />
   *             <TextField source="author" />
   *             <TextField source="year" textAlign="right" />
   *         </DataTable>
   *     </List>
   * );
   */
  textAlign?: TextAlign;

  /**
   * The component to display when the field value is empty. Defaults to empty string.
   *
   * @example
   * const PostList = () => (
   *     <List>
   *         <DataTable>
   *             <TextField source="title" />
   *             <TextField source="author" empty="missing data" />
   *         </DataTable>
   *     </List>
   * );
   */
  empty?: ReactNode;
}
