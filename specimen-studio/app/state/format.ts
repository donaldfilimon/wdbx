export const formText = (form: FormData, key: string): string => {
  const v = form.get(key);
  if (typeof v !== 'string') throw new Error(`Missing field: ${key}`);
  return v;
};

export const fmt = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 1,
});

export const clock = (s: string) =>
  new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(s));
