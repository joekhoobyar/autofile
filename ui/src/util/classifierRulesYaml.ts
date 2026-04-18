import { parseDocument, stringify } from 'yaml';

import { type ClassifierBlock, type ClassifierModifier, type ClassifierRules } from '../models/classifierBlock';

export const defaultClassifierRules: ClassifierRules = {
  continue_after_match: false,
  match_patterns: [],
  match_actions: {},
  child_rules: [],
};

type ParseResult = {
  value?: ClassifierRules;
  error?: string;
};

type ModifierType = ClassifierModifier['type'];

const modifierTypes = new Set<ModifierType>([
  'metadata',
  'month_number',
  'month_end',
  'month_start',
  'next_day',
  'prev_day',
  'next_month',
  'prev_month',
  'tax_year',
  'currency',
  'sprintf',
  'replace',
  'alnum_sanitize',
  'date_format',
  'add',
  'sub',
  'mul',
  'div',
]);

function cloneDefaultRules(): ClassifierRules {
  return {
    continue_after_match: false,
    match_patterns: [],
    match_actions: {},
    child_rules: [],
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function validateString(value: unknown, path: string): string | null {
  return typeof value === 'string' ? null : `${path} must be a string`;
}

function validateNumber(value: unknown, path: string): string | null {
  return typeof value === 'number' ? null : `${path} must be a number`;
}

function validateStringMap(value: unknown, path: string): string | null {
  if (!isRecord(value)) {
    return `${path} must be an object`;
  }

  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry !== 'string') {
      return `${path}.${key} must be a string`;
    }
  }

  return null;
}

function validatePattern(value: unknown, path: string): string | null {
  if (!isRecord(value)) {
    return `${path} must be an object`;
  }

  if (value.text !== undefined && value.text !== null) {
    const textError = validateString(value.text, `${path}.text`);
    if (textError) return textError;
  }

  if (value.metadata !== undefined && value.metadata !== null) {
    const metadataError = validateStringMap(value.metadata, `${path}.metadata`);
    if (metadataError) return metadataError;
  }

  return null;
}

function validateModifier(value: unknown, path: string): string | null {
  if (!isRecord(value)) {
    return `${path} must be an object`;
  }

  const { type } = value;
  if (typeof type !== 'string' || !modifierTypes.has(type as ModifierType)) {
    return `${path}.type must be a supported modifier type`;
  }

  switch (type as ModifierType) {
    case 'metadata':
      return validateNumber(value.to, `${path}.to`) ?? validateString(value.slug, `${path}.slug`);
    case 'sprintf':
    case 'date_format':
      return validateNumber(value.to, `${path}.to`) ?? validateString(value.from, `${path}.from`) ?? validateString(value.format, `${path}.format`);
    case 'month_end':
    case 'month_start':
    case 'next_day':
    case 'prev_day':
    case 'next_month':
    case 'prev_month':
    case 'tax_year':
    case 'replace':
    case 'alnum_sanitize':
    case 'month_number':
    case 'currency':
      return validateString(value.from, `${path}.from`) ?? validateNumber(value.to, `${path}.to`);
    case 'add':
    case 'sub':
    case 'mul':
    case 'div':
      return validateNumber(value.from, `${path}.from`) ?? validateNumber(value.to, `${path}.to`);
    default:
      return `${path}.type must be a supported modifier type`;
  }
}

function validateChildRule(value: unknown, path: string): string | null {
  if (!isRecord(value)) {
    return `${path} must be an object`;
  }

  const patternError = validatePattern(value.pattern, `${path}.pattern`);
  if (patternError) return patternError;

  if (value.modifiers !== undefined && value.modifiers !== null) {
    if (!Array.isArray(value.modifiers)) {
      return `${path}.modifiers must be an array`;
    }

    for (let index = 0; index < value.modifiers.length; index += 1) {
      const modifierError = validateModifier(value.modifiers[index], `${path}.modifiers[${index}]`);
      if (modifierError) return modifierError;
    }
  }

  return validateStringMap(value.actions, `${path}.actions`);
}

export function validateClassifierRules(value: unknown): string | null {
  if (!isRecord(value)) {
    return 'rules must be an object';
  }

  if (!Array.isArray(value.match_patterns)) {
    return 'rules.match_patterns must be an array';
  }

  if (value.continue_after_match !== undefined && typeof value.continue_after_match !== 'boolean') {
    return 'rules.continue_after_match must be a boolean';
  }

  for (let index = 0; index < value.match_patterns.length; index += 1) {
    const patternError = validatePattern(value.match_patterns[index], `rules.match_patterns[${index}]`);
    if (patternError) return patternError;
  }

  const actionError = validateStringMap(value.match_actions, 'rules.match_actions');
  if (actionError) return actionError;

  if (!Array.isArray(value.child_rules)) {
    return 'rules.child_rules must be an array';
  }

  for (let index = 0; index < value.child_rules.length; index += 1) {
    const childRuleError = validateChildRule(value.child_rules[index], `rules.child_rules[${index}]`);
    if (childRuleError) return childRuleError;
  }

  return null;
}

export function rulesToYaml(rules?: ClassifierRules | ClassifierBlock['rules']): string {
  return stringify(rules ?? cloneDefaultRules(), {
    indent: 2,
  }).trim();
}

export function yamlToRules(text: string): ParseResult {
  const trimmed = text.trim();
  if (!trimmed) {
    return { value: cloneDefaultRules() };
  }

  const doc = parseDocument(text);
  if (doc.errors.length > 0) {
    return { error: doc.errors[0].message };
  }

  const value = doc.toJS();
  const validationError = validateClassifierRules(value);
  if (validationError) {
    return { error: validationError };
  }

  return {
    value: {
      continue_after_match: value.continue_after_match === true,
      match_patterns: value.match_patterns as ClassifierRules['match_patterns'],
      match_actions: value.match_actions as ClassifierRules['match_actions'],
      child_rules: value.child_rules as ClassifierRules['child_rules'],
    },
  };
}
