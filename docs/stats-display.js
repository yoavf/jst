export function statsTotalSizeStep(formattedValue) {
  const digitCount = String(formattedValue).match(/\d/g)?.length ?? 0;
  return Math.min(Math.max(digitCount - 4, 0), 5);
}
