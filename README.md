# Napp — verhuisd naar de monorepo

> **Deze repository is gearchiveerd.** De Napp-poc leeft verder in
> [MinBZK/regelrecht](https://github.com/MinBZK/regelrecht) en is te bereiken
> via [poc.regelrecht.rijks.app/napp](https://poc.regelrecht.rijks.app/napp/)
> (achter een wachtwoord).

Wat hier nog staat is één ding: een permanente verwijzing (301) van
`napp-poc.rijks.app` naar de poc op zijn nieuwe plek, zodat rondgestuurde
links blijven werken. De code daarvan staat in `redirect/`.

Het oude adres serveerde de casus aan iedereen die de link had, terwijl het
portaal er een wachtwoord voor zet. Dat is de reden dat het niet gewoon is
blijven draaien.

## Waar het nu staat

| Was | Is |
|---|---|
| `backend/` | `packages/poc-napp/` |
| `frontend/` | `frontend-poc-napp/` |
| `law/` | `corpus-poc/napp/law/` |
| `scenarios/` | `corpus-poc/napp/scenarios/` |
| `scripts/` | `packages/poc-napp/scripts/` |

De uitleg die in deze README stond (de twee organisatiemodellen uit de MvT bij
art. 27, de besluitstate, de datumketen AWB 6:8 naar de Algemene termijnenwet)
staat nu in `packages/poc-napp/README.md` in de monorepo.

## De geschiedenis

De commits tot en met juni 2026 zijn het werk aan de losse PoC. Die historie
is hier bewaard; de monorepo begint bij de migratie. Eén lokale branch met een
uitwerking van de aanvraagvereisten is bewust niet meegegaan: die voerde een
artikelnummer op dat het wetsvoorstel niet kent.
