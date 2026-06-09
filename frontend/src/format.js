/** Formatting helpers (Dutch locale). */

const euroFormat = new Intl.NumberFormat('nl-NL', {
  style: 'currency',
  currency: 'EUR',
});

/** Format an amount in eurocent as a euro string. */
export function euro(cents) {
  return euroFormat.format((cents ?? 0) / 100);
}

const dateFormat = new Intl.DateTimeFormat('nl-NL', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
});

/** Format an ISO date (YYYY-MM-DD) in Dutch. */
export function datum(iso) {
  if (!iso) return '';
  const d = new Date(`${iso.slice(0, 10)}T00:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  return dateFormat.format(d);
}

/** Human-readable status of an application. */
export function statusLabel(status, besluit) {
  switch (status) {
    case 'BEHANDELING':
      return 'In behandeling';
    case 'BESLUIT':
      return besluit?.subsidie_toegekend ? 'Toegekend' : 'Afgewezen';
    case 'BEZWAAR':
      return besluit?.subsidie_toegekend
        ? 'Toegekend · bezwaartermijn loopt'
        : 'Afgewezen · bezwaartermijn loopt';
    default:
      return status;
  }
}

/** Tag color for a status. */
export function statusColor(status, besluit) {
  if (status === 'BEHANDELING') return 'warning';
  if (besluit && !besluit.subsidie_toegekend) return 'critical';
  if (status === 'BESLUIT' || status === 'BEZWAAR') return 'success';
  return 'neutral';
}

export function onderdelen(n) {
  return `${n} ${n === 1 ? 'onderdeel' : 'onderdelen'}`;
}
