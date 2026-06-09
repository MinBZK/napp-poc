# Napp — Nederlandse autoriteit politieke partijen

End-to-end demonstratie van wetsuitvoering als code: het subsidieproces van de
Nederlandse autoriteit politieke partijen (Napp) uit het wetsvoorstel
[Wet op de politieke partijen](https://www.tweedekamer.nl/kamerstukken/wetsvoorstellen/detail?qry=wetsvoorstel%3A36742)
(kamerstuk 36742), uitgevoerd door de
[regelrecht](https://github.com/MinBZK/regelrecht)-engine.

De wet is een uitvoerbare reconstructie (werkversie); zodra de definitieve
tekst er is, wordt het YAML-bestand vervangen zonder dat de rest verandert.

## Wat zit erin

| Onderdeel | Beschrijving |
|---|---|
| `law/` | Machine-leesbare wetten: Wpp (twee subsidietracks + betaalopdracht-hook), regeling met bedragen (IoC), AWB-subset (procedure + hooks 3:46/6:7/6:8), Algemene termijnenwet (weekend-verlenging) |
| `scenarios/` | 21 Gherkin-scenario's die vastleggen hoe de wet hoort te werken |
| `backend/` | Axum-orchestratielaag: aanvragen, besluiten (RFC-008 besluit-state), betaalopdrachten, openbaar register; SQLite; SSO Rijk via regelrecht-auth (OIDC) met demo-fallback |
| `frontend/` | Drie gescheiden ingangen (Vue 3 + NLDD design system): publiek (landing + register), subsidieportaal voor partijen (mock-eHerkenning), beoordelingsomgeving voor de Napp (incl. in-browser scenario-runner op de WASM-engine) |

## Draaien

Vereist: Rust (incl. `wasm32-unknown-unknown` target + `wasm-bindgen`),
Node.js, [just](https://github.com/casey/just), en een checkout van
[regelrecht](https://github.com/MinBZK/regelrecht) naast deze repo
(of zet `REGELRECHT_DIR`).

```bash
just law-validate   # wetten valideren tegen het regelrecht-schema
just bdd            # 21 scenario's op de Rust-engine
just wasm           # engine naar WASM bouwen (voor de scenario-runner)
cd frontend && npm install
just dev            # backend (:8400) + frontend (:5400)
```

Ingangen:

- http://localhost:5400/ — publieke site + openbaar register
- http://localhost:5400/aanvrager/ — subsidieportaal (eHerkenning gemockt)
- http://localhost:5400/beoordelaar/ — beoordelingsomgeving (SSO Rijk, of
  demo-login zonder OIDC-configuratie)

SSO Rijk wordt actief zodra de `OIDC_*`-omgevingsvariabelen zijn gezet
(zelfde configuratie als regelrecht's editor-api).

## Hoe het werkt

1. Een partij dient een aanvraag in; de backend persisteert de besluit-state
   (stage `BEHANDELING`, conform RFC-008: de engine is stateless, de
   orchestratielaag bewaart de toestand).
2. De beoordelaar ziet de uitkomst die de wet berekent (recht, bedrag,
   motivering met artikelverwijzingen) en stelt het besluit vast. Bij
   toekenning vuurt artikel 16 (post_actions-hook) en ontstaat een
   betaalopdracht naar het (gemockte) betaalsysteem.
3. Bij bekendmaking berekent AWB 6:8 de bezwaartermijn; de Algemene
   termijnenwet verlengt een einddatum die in het weekend valt naar de
   eerstvolgende werkdag (feestdagen zijn als untranslatable gemarkeerd,
   RFC-012).
4. Het bekendgemaakte besluit verschijnt in het openbare register met
   statistieken.

De scenario-runner in de beoordelingsomgeving draait dezelfde 21 scenario's
live in de browser op de naar WASM gecompileerde engine: het bewijs dat de
wet doet wat hij moet doen, naast elke beoordeling.
