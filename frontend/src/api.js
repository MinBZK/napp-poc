/**
 * API client for the NAPP backend. Session-cookie based; all requests go
 * through the Vite dev proxy (or same origin in production).
 */

async function handle(response) {
  if (!response.ok) {
    let message = `HTTP ${response.status}`;
    try {
      const body = await response.json();
      if (body.fout) message = body.fout;
    } catch {
      // geen JSON-body
    }
    throw new Error(message);
  }
  return response.json();
}

export function apiGet(path) {
  return fetch(path, { credentials: 'include' }).then(handle);
}

export function apiPost(path, data) {
  return fetch(path, {
    method: 'POST',
    headers: data ? { 'Content-Type': 'application/json' } : undefined,
    body: data ? JSON.stringify(data) : undefined,
    credentials: 'include',
  }).then(handle);
}

export function apiPut(path, data) {
  return fetch(path, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
    credentials: 'include',
  }).then(handle);
}

export function apiDelete(path) {
  return fetch(path, { method: 'DELETE', credentials: 'include' }).then(handle);
}

export const api = {
  me: () => apiGet('/api/me'),
  eherkenningLogin: (kvkNummer, machtiging) =>
    apiPost(
      '/api/eherkenning/login',
      machtiging
        ? { kvk_nummer: kvkNummer, machtiging }
        : { kvk_nummer: kvkNummer },
    ),
  eherkenningMachtigingen: (kvkNummer) =>
    apiGet(`/api/eherkenning/machtigingen?kvk=${encodeURIComponent(kvkNummer)}`),
  eherkenningLogout: () => apiPost('/api/eherkenning/logout'),
  mijnRegistratie: () => apiGet('/api/mijn-registratie'),
  registerDemo: () => apiGet('/api/register/demo'),
  // Claim-flow: een rechtspersoon koppelt zichzelf aan een ongekoppelde
  // aanduiding uit de verkiezingsuitslag.
  claimAanduidingen: (zoek = '') =>
    apiGet(`/api/claim/aanduidingen?zoek=${encodeURIComponent(zoek)}`),
  claimIndienen: (doelKvk) => apiPost('/api/claim', { doel_kvk: doelKvk }),
  mijnClaim: () => apiGet('/api/mijn-claim'),
  ssoMockLogin: (naam) => apiPost('/api/sso/mock-login', { naam }),
  aanvragen: () => apiGet('/api/aanvragen'),
  aanvraag: (id) => apiGet(`/api/aanvragen/${id}`),
  mijnAanvragen: () => apiGet('/api/mijn-aanvragen'),
  mijnAanvraag: (id) => apiGet(`/api/mijn-aanvragen/${id}`),
  nieuweAanvraag: (payload) => apiPost('/api/aanvragen', payload),
  proefAanspraken: (payload) => apiPost('/api/aanvragen/proef', payload),
  proefberekening: (id) => apiPost(`/api/aanvragen/${id}/proefberekening`),
  stelBesluitVast: (id) => apiPost(`/api/aanvragen/${id}/besluit`),
  bekendmaking: (id) => apiPost(`/api/aanvragen/${id}/bekendmaking`),
  betaalopdrachten: () => apiGet('/api/betaalopdrachten'),
  register: () => apiGet('/api/register'),
  statistieken: () => apiGet('/api/register/statistieken'),
  // Partijregister-beheer (beoordelaar-only).
  beheerPartijen: ({ zoek = '', offset = 0, limit = 25 } = {}) =>
    apiGet(`/api/beheer/partijen?${new URLSearchParams({ zoek, offset, limit })}`),
  beheerPartij: (kvk) => apiGet(`/api/beheer/partijen/${kvk}`),
  beheerPartijWijzigen: (kvk, payload) => apiPut(`/api/beheer/partijen/${kvk}`, payload),
  beheerClaims: () => apiGet('/api/beheer/claims'),
  beheerClaimBevestigen: (id) => apiPost(`/api/beheer/claims/${id}/bevestig`),
  beheerClaimAfwijzen: (id, reden) => apiPost(`/api/beheer/claims/${id}/afwijzen`, { reden }),
};
