Feature: Subsidie voor landelijke politieke partijen
  Als landelijke politieke partij met ten minste een kamerzetel en duizend
  betalende leden wil ik subsidie ontvangen van de Nederlandse autoriteit
  politieke partijen, berekend als basisbedrag plus bedrag per kamerzetel
  plus bedrag per lid.

  Background:
    Given the calculation date is "2026-06-01"

  Scenario: Middelgrote landelijke partij met 10 zetels en 5000 leden
    Given an application with the following data:
      | niveau                           | LANDELIJK |
      | aantal_kamerzetels               | 10        |
      | aantal_betalende_leden           | 5000      |
      | aantal_raadszetels               | 0         |
      | inwoneraantal_gemeente           | 0         |
      | ontvangt_anonieme_giften         | false     |
      | ontvangt_giften_niet_ingezetenen | false     |
      | voldoet_aan_meldplicht_giften    | true      |
      | financien_openbaar_op_website    | true      |
    When the subsidiebesluit is executed
    Then the subsidie is toegekend
    And the subsidiebedrag is "105721100" eurocent
    And a betaalopdracht of "105721100" eurocent is required

  Scenario: Kleine partij precies op de drempel van 1 zetel en 1000 leden
    Given an application with the following data:
      | niveau                           | LANDELIJK |
      | aantal_kamerzetels               | 1         |
      | aantal_betalende_leden           | 1000      |
      | aantal_raadszetels               | 0         |
      | inwoneraantal_gemeente           | 0         |
      | ontvangt_anonieme_giften         | false     |
      | ontvangt_giften_niet_ingezetenen | false     |
      | voldoet_aan_meldplicht_giften    | true      |
      | financien_openbaar_op_website    | true      |
    When the subsidiebesluit is executed
    Then the subsidie is toegekend
    And the subsidiebedrag is "32776900" eurocent

  Scenario: Grote partij met 30 zetels en 50000 leden
    Given an application with the following data:
      | niveau                           | LANDELIJK |
      | aantal_kamerzetels               | 30        |
      | aantal_betalende_leden           | 50000     |
      | aantal_raadszetels               | 0         |
      | inwoneraantal_gemeente           | 0         |
      | ontvangt_anonieme_giften         | false     |
      | ontvangt_giften_niet_ingezetenen | false     |
      | voldoet_aan_meldplicht_giften    | true      |
      | financien_openbaar_op_website    | true      |
    When the subsidiebesluit is executed
    Then the subsidie is toegekend
    And the subsidiebedrag is "358097100" eurocent
