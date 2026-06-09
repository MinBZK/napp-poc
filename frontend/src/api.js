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

export const api = {
  me: () => apiGet('/api/me'),
  eherkenningLogin: (kvkNummer, partijNaam) =>
    apiPost('/api/eherkenning/login', { kvk_nummer: kvkNummer, partij_naam: partijNaam }),
  eherkenningLogout: () => apiPost('/api/eherkenning/logout'),
  ssoMockLogin: (naam) => apiPost('/api/sso/mock-login', { naam }),
  aanvragen: () => apiGet('/api/aanvragen'),
  aanvraag: (id) => apiGet(`/api/aanvragen/${id}`),
  nieuweAanvraag: (payload) => apiPost('/api/aanvragen', payload),
  proefberekening: (id) => apiPost(`/api/aanvragen/${id}/proefberekening`),
  stelBesluitVast: (id) => apiPost(`/api/aanvragen/${id}/besluit`),
  bekendmaking: (id) => apiPost(`/api/aanvragen/${id}/bekendmaking`),
  betaalopdrachten: () => apiGet('/api/betaalopdrachten'),
  register: () => apiGet('/api/register'),
  statistieken: () => apiGet('/api/register/statistieken'),
};
