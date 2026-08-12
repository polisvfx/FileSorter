import type { Rule } from '$lib/types';

let nextId = $state(1);
let rules = $state<Rule[]>([]);

export function getRules(): Rule[] {
  return rules;
}

export function setRules(newRules: Partial<Rule>[]) {
  if (newRules.length > 0) {
    nextId = Math.max(...newRules.map((r) => r.id ?? 0)) + 1;
  }
  // Presets and sessions saved before regex/scope existed are missing those
  // fields; fill them in rather than letting undefined reach the backend.
  rules = newRules.map(withDefaults);
}

export function addRule() {
  rules.push(blankRule());
}

function blankRule(): Rule {
  return {
    id: nextId++,
    contains: '',
    contains_not: null,
    target_folder: '',
    enabled: true,
    stop_on_match: false,
    regex: false,
    case_sensitive: false,
    scope: 'name'
  };
}

/** Copy a rule directly below the original — building a set of near-identical
 *  rules by hand was the most common way to fill the list. */
export function duplicateRule(id: number) {
  const index = rules.findIndex((r) => r.id === id);
  if (index < 0) return;
  rules.splice(index + 1, 0, { ...rules[index], id: nextId++ });
}

/** Fill in fields added after a preset or session was saved. */
export function withDefaults(rule: Partial<Rule>): Rule {
  return { ...blankRule(), ...rule, id: rule.id ?? nextId++ };
}

export function removeRule(id: number) {
  rules = rules.filter((r) => r.id !== id);
}

export function updateRule(id: number, field: keyof Rule, value: string | boolean | null) {
  const rule = rules.find((r) => r.id === id);
  if (rule) {
    (rule as any)[field] = value;
  }
}

export function reorderRules(newOrder: Rule[]) {
  rules = newOrder;
}
