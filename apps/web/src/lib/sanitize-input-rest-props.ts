import type { UnknownRecord } from "./unknown-types";

/**
 * Strip ra-core input-controller props before spreading the remaining
 * props onto a DOM element. Mirrors the upstream
 * `sanitizeInputRestProps` from `ra-core`.
 */
export const sanitizeInputRestProps = ({
  afterSubmit: _afterSubmit,
  allowNull: _allowNull,
  alwaysOn: _alwaysOn,
  beforeSubmit: _beforeSubmit,
  component: _component,
  data: _data,
  defaultValue: _defaultValue,
  error: _error,
  format: _format,
  formatOnBlur: _formatOnBlur,
  initializeForm: _initializeForm,
  input: _input,
  isEqual: _isEqual,
  isRequired: _isRequired,
  label: _label,
  limitChoicesToValue: _limitChoicesToValue,
  locale: _locale,
  meta: _meta,
  multiple: _multiple,
  name: _name,
  options: _options,
  optionText: _optionText,
  optionValue: _optionValue,
  parse: _parse,
  record: _record,
  ref: _ref,
  refetch: _refetch,
  render: _render,
  resource: _resource,
  setFilter: _setFilter,
  setPagination: _setPagination,
  setSort: _setSort,
  shouldUnregister: _shouldUnregister,
  source: _source,
  submitError: _submitError,
  subscription: _subscription,
  textAlign: _textAlign,
  translate: _translate,
  translateChoice: _translateChoice,
  validate: _validate,
  validateFields: _validateFields,
  value: _value,
  ...rest
}: UnknownRecord) => rest;
