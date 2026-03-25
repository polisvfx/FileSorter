import type { Rule } from '$lib/types';

let nextId = $state(1);
let rules = $state<Rule[]>([]);

export function getRules(): Rule[] {
  return rules;
}

export function setRules(newRules: Rule[]) {
  rules = newRules;
  if (newRules.length > 0) {
    nextId = Math.max(...newRules.map((r) => r.id)) + 1;
  }
}

export function addRule() {
  rules.push({
    id: nextId++,
    contains: '',
    contains_not: null,
    target_folder: ''
  });
}

export function removeRule(id: number) {
  rules = rules.filter((r) => r.id !== id);
}

export function updateRule(id: number, field: keyof Rule, value: string | null) {
  const rule = rules.find((r) => r.id === id);
  if (rule) {
    (rule as any)[field] = value;
  }
}

export function reorderRules(newOrder: Rule[]) {
  rules = newOrder;
}
