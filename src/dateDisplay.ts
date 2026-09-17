const asLocalDate = (value: string) => {
  const normalized = value.includes('T') ? value : `${value}T00:00:00`;
  const date = new Date(normalized);
  return Number.isNaN(date.getTime()) ? null : date;
};

export function fullGregorianDate(value?: string | null) {
  if (!value) return 'غير مسجل';
  const date = asLocalDate(value);
  if (!date) return value;
  const text = date.toLocaleDateString('ar-SA-u-ca-gregory', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
  return `${text} • الشهر ${String(date.getMonth() + 1).padStart(2, '0')}`;
}

export function fullGregorianDateTime(value?: string | null) {
  if (!value) return 'غير مسجل';
  const date = asLocalDate(value);
  if (!date) return value;
  const text = date.toLocaleString('ar-SA-u-ca-gregory', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
  return `${text} • الشهر ${String(date.getMonth() + 1).padStart(2, '0')}`;
}
