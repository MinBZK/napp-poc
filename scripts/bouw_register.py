#!/usr/bin/env python3
"""Bouwt backend/data/partijregister.json uit open data.

Bronnen:
- Verkiezingsuitslag Tweede Kamer 2025 (Kiesraad, data.overheid.nl, CSV):
  landelijke partijen met kamerzetels.
- Verkiezingsuitslagen Gemeenteraad 2026 (Kiesraad, data.overheid.nl, CSV):
  raadszetels per lijst per gemeente.
- CBS StatLine 37230ned (OData): inwoneraantal per gemeente (januari 2026).

De KvK-nummers zijn synthetisch (de koppeling rechtspersoon-aanduiding is
geen open data); ze worden deterministisch afgeleid zodat het register
reproduceerbaar is. Landelijke lijsten zijn één partij voor alle gemeenten;
lokale lijsten zijn een eigen partij per gemeente (dezelfde naam in twee
gemeenten is in werkelijkheid ook een andere partij).

Decentrale orgaan-typen: het datamodel kent GEMEENTERAAD, PROVINCIALE_STATEN
en WATERSCHAP. Gemeenteraden zijn gevuld uit GR2026. Voor provinciale staten
(PS2023) en waterschappen (AB2023) publiceert de Kiesraad alleen
stemmen-CSV's zonder zetels ("onderzoeksdata"); zetels staan in de EML's of
zijn met D'Hondt uit de stemmen af te leiden (PS: wettelijke zetelaantallen
per provincie, Kieswet C 2). Dat is een vervolgstap; de structuur is er al.

Draaien: uv run scripts/bouw_register.py
"""

import csv
import hashlib
import io
import json
import urllib.request
import zipfile
from pathlib import Path

TK2025_CSV_ZIP = (
    "https://data.overheid.nl/sites/default/files/dataset/"
    "a16f3352-c9ce-4831-a314-f989d442a258/resources/"
    "Verkiezingsuitslag%20Tweede%20Kamer%202025%20%28CSV%20Formaat%29.zip"
)
GR2026_CSV = (
    "https://data.overheid.nl/sites/default/files/dataset/"
    "09cf04e7-11ac-4b53-a2cc-baccf95e82fd/resources/GR2026.csv"
)
CBS_BEVOLKING = (
    "https://opendata.cbs.nl/ODataApi/odata/37230ned/TypedDataSet"
    "?$filter=startswith(RegioS,'GM')%20and%20Perioden%20eq%20'2026MM01'"
    "&$select=RegioS,BevolkingAanHetBeginVanDePeriode_1&$top=10000&$format=json"
)

UIT = Path(__file__).resolve().parent.parent / "backend" / "data" / "partijregister.json"


def haal(url: str) -> bytes:
    print(f"downloaden: {url[:90]}...")
    with urllib.request.urlopen(url) as response:
        return response.read()


def kvk_voor(sleutel: str) -> str:
    """Deterministisch synthetisch KvK-nummer (8 cijfers, begint met 9)."""
    digest = hashlib.sha1(sleutel.encode()).hexdigest()
    return "9" + str(int(digest, 16) % 10_000_000).zfill(7)


def main() -> None:
    # --- TK2025: landelijke zetels ---
    zipdata = zipfile.ZipFile(io.BytesIO(haal(TK2025_CSV_ZIP)))
    naam_uitslag = next(n for n in zipdata.namelist() if n.endswith("TK2025_uitslag.csv"))
    landelijk: dict[str, int] = {}
    with zipdata.open(naam_uitslag) as f:
        tekst = io.TextIOWrapper(f, encoding="utf-8-sig")
        for rij in csv.DictReader(tekst, delimiter=";"):
            if rij["Regio"] == "Nederland" and rij["VeldType"] == "LijstAantalZetels":
                landelijk[rij["LijstNaam"]] = int(rij["Waarde"])
    print(f"TK2025: {len(landelijk)} landelijke partijen met zetels")

    # --- GR2026: raadszetels per gemeente ---
    gemeente_namen: dict[str, str] = {}
    uitslagen: list[tuple[str, str, int]] = []  # (gemeentecode, lijstnaam, zetels)
    tekst = io.StringIO(haal(GR2026_CSV).decode("utf-8-sig"))
    for rij in csv.DictReader(tekst, delimiter=";"):
        if rij["VeldType"] != "LijstAantalZetels":
            continue
        code = rij["RegioCode"]
        if not code.startswith("G"):
            continue
        gm = "GM" + code[1:]
        gemeente_namen[gm] = rij["Regio"]
        uitslagen.append((gm, rij["LijstNaam"], int(rij["Waarde"])))
    print(f"GR2026: {len(uitslagen)} uitslagen in {len(gemeente_namen)} gemeenten")

    # --- CBS: inwoneraantal per gemeente ---
    cbs = json.loads(haal(CBS_BEVOLKING))
    inwoners = {
        rij["RegioS"].strip(): rij["BevolkingAanHetBeginVanDePeriode_1"]
        for rij in cbs["value"]
        if rij["BevolkingAanHetBeginVanDePeriode_1"] is not None
    }
    print(f"CBS: inwoneraantallen voor {len(inwoners)} gemeenten")

    # --- Partijen samenstellen ---
    partijen: dict[str, dict] = {}
    for naam, zetels in sorted(landelijk.items()):
        kvk = kvk_voor(f"landelijk|{naam}")
        partijen[kvk] = {
            "kvk_nummer": kvk,
            "naam": naam,
            "kamerzetels": zetels,
            "decentrale_uitslagen": [],
        }

    landelijke_namen = set(landelijk)
    for gm, lijstnaam, zetels in uitslagen:
        if zetels == 0:
            continue
        if lijstnaam in landelijke_namen:
            kvk = kvk_voor(f"landelijk|{lijstnaam}")
        else:
            kvk = kvk_voor(f"lokaal|{gm}|{lijstnaam}")
            if kvk not in partijen:
                partijen[kvk] = {
                    "kvk_nummer": kvk,
                    "naam": lijstnaam,
                    "kamerzetels": 0,
                    "decentrale_uitslagen": [],
                }
        partijen[kvk]["decentrale_uitslagen"].append(
            {"orgaan": "GEMEENTERAAD", "gebied_code": gm, "zetels": zetels}
        )

    gebieden = [
        {
            "orgaan": "GEMEENTERAAD",
            "code": gm,
            "naam": naam,
            "inwoneraantal": inwoners.get(gm, 0),
        }
        for gm, naam in sorted(gemeente_namen.items(), key=lambda x: x[1])
    ]

    # --- Demo-voorbeelden voor de mock-login ---
    grootste = max(landelijk, key=landelijk.get)
    kleinste = min(landelijk, key=landelijk.get)
    lokaal = max(
        (p for p in partijen.values() if p["kamerzetels"] == 0 and p["decentrale_uitslagen"]),
        key=lambda p: max(u["zetels"] for u in p["decentrale_uitslagen"]),
    )
    demo = [
        {"kvk_nummer": kvk_voor(f"landelijk|{grootste}"), "naam": grootste},
        {"kvk_nummer": kvk_voor(f"landelijk|{kleinste}"), "naam": kleinste},
        {"kvk_nummer": lokaal["kvk_nummer"], "naam": lokaal["naam"]},
    ]

    register = {
        "bronnen": {
            "landelijk": "Verkiezingsuitslag Tweede Kamer 2025 (Kiesraad, data.overheid.nl)",
            "decentraal": "Verkiezingsuitslagen Gemeenteraad 2026 (Kiesraad, data.overheid.nl)",
            "inwoneraantallen": "CBS StatLine 37230ned, januari 2026",
            "kvk_nummers": "synthetisch (koppeling rechtspersoon-aanduiding is geen open data)",
        },
        "partijen": sorted(partijen.values(), key=lambda p: (-p["kamerzetels"], p["naam"])),
        "gebieden": gebieden,
        "demo_voorbeelden": demo,
    }

    UIT.parent.mkdir(parents=True, exist_ok=True)
    UIT.write_text(json.dumps(register, ensure_ascii=False, indent=1))
    print(f"geschreven: {UIT} ({UIT.stat().st_size // 1024} kB, "
          f"{len(partijen)} partijen, {len(gebieden)} gebieden)")
    for d in demo:
        print("  demo-login:", d["kvk_nummer"], "→", d["naam"])


if __name__ == "__main__":
    main()
