export function dimBarColor(key: string): string {
  const map: Record<string, string> = {
    goal: 'bg-blue-500',
    'business-line': 'bg-amber-500',
    'importance-urgency': 'bg-pink-500',
    category: 'bg-green-500',
  };
  return map[key] || 'bg-gray-400';
}

export function getValueCount(
  dimensions: Array<{ key: string; source?: string; values?: string[] }>,
  key: string,
  monthlyGoalOptions: string[],
): number {
  const dim = dimensions.find((d) => d.key === key);
  if (!dim) return 0;
  if (dim.source === 'monthly') return monthlyGoalOptions.length;
  return dim.values?.length || 0;
}

export function firstUnfilledRequiredIndex(
  items: Array<{ key?: string; required?: boolean }>,
  dimValues: Record<string, string>,
): number {
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (item.required && (!item.key || !dimValues[item.key])) {
      return i;
    }
  }
  return 0;
}
