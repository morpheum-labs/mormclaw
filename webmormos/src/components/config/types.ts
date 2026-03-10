import type { LucideIcon } from 'lucide-react';

export type FieldType =
  | 'text'
  | 'password'
  | 'number'
  | 'toggle'
  | 'select'
  | 'tag-list';

export interface FieldDef {
  key: string;
  label: string;
  type: FieldType;
  description?: string;
  /** When set, use t(labelKey) for display instead of label. */
  labelKey?: string;
  sensitive?: boolean;
  defaultValue?: unknown;
  options?: { value: string; label: string }[];
  min?: number;
  max?: number;
  step?: number;
  tagPlaceholder?: string;
}

export interface SectionDef {
  path: string;
  title: string;
  /** When set, use t(titleKey) for display instead of title. */
  titleKey?: string;
  description?: string;
  icon: LucideIcon;
  fields: FieldDef[];
  defaultCollapsed?: boolean;
  category?: string;
}

export interface FieldProps {
  field: FieldDef;
  value: unknown;
  onChange: (value: unknown) => void;
  isMasked: boolean;
}
