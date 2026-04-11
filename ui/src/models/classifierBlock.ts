export interface ClassifierPattern {
  text: string;
  metadata: Record<string, string>;
}

export type ClassifierModifier =
  | { type: 'metadata'; to: number; slug: string }
  | { type: 'month_number'; from: number; to: number }
  | { type: 'month_end'; from: string; to: number }
  | { type: 'month_start'; from: string; to: number }
  | { type: 'next_day'; from: string; to: number }
  | { type: 'prev_day'; from: string; to: number }
  | { type: 'next_month'; from: string; to: number }
  | { type: 'prev_month'; from: string; to: number }
  | { type: 'tax_year'; from: string; to: number }
  | { type: 'currency'; from: number; to: number }
  | { type: 'sprintf'; from: number; to: number; format: string }
  | { type: 'replace'; from: string; to: number }
  | { type: 'alnum_sanitize'; from: string; to: number }
  | { type: 'date_format'; from: string; to: number; format: string }
  | { type: 'add'; from: number; to: number }
  | { type: 'sub'; from: number; to: number }
  | { type: 'mul'; from: number; to: number }
  | { type: 'div'; from: number; to: number };

export interface ClassifierChildRule {
  pattern: ClassifierPattern;
  modifiers: ClassifierModifier[];
  actions: Record<string, string>;
}

export interface ClassifierRules {
  match_patterns: ClassifierPattern[];
  match_actions: Record<string, string>;
  child_rules: ClassifierChildRule[];
}

export interface ClassifierBlock {
  id: number;
  name: string;
  description?: string;
  enabled: boolean;
  order: number;
  rules: ClassifierRules;
}
