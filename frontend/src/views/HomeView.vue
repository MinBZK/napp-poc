<script setup>
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../components/PortalHeader.vue';
import { api } from '../api.js';
import { euro } from '../format.js';

const router = useRouter();
const stats = ref(null);

const navItems = [
  { text: 'Home', to: '/' },
  { text: 'Openbaar register', to: '/register' },
];

onMounted(async () => {
  try {
    stats.value = await api.statistieken();
  } catch {
    stats.value = null;
  }
});
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Nederlandse autoriteit politieke partijen"
      :items="navItems"
    />

    <nldd-simple-section>
      <nldd-title size="1">
        <span slot="overline">Wet op de politieke partijen</span>
        <h1>Subsidies voor politieke partijen, uitgevoerd als wet</h1>
      </nldd-title>
      <nldd-spacer size="16"></nldd-spacer>
      <nldd-rich-text>
        <p>
          De Nederlandse autoriteit politieke partijen (Napp) verstrekt subsidies aan
          landelijke en decentrale politieke partijen. De voorwaarden en de hoogte van
          de subsidie volgen rechtstreeks uit de wet: elke aanvraag wordt beoordeeld
          door de wet zelf uit te voeren, machine-leesbaar en controleerbaar.
        </p>
      </nldd-rich-text>
    </nldd-simple-section>

    <nldd-simple-section background="tinted">
      <nldd-title size="2" slot="header">
        <h2>Eén wet, drie ingangen</h2>
      </nldd-title>
      <nldd-collection layout="grid" item-width="300px">
        <nldd-card accessible-label="Politieke partijen">
          <nldd-container padding="20">
            <nldd-icon name="apartment-building" size="32" color="lintblauw"></nldd-icon>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-title size="4"><h3>Politieke partijen</h3></nldd-title>
            <nldd-spacer size="8"></nldd-spacer>
            <nldd-rich-text>
              <p>
                Vraag subsidie aan met eHerkenning in het subsidieportaal en volg uw
                aanvraag door het hele besluitproces.
              </p>
            </nldd-rich-text>
            <nldd-spacer size="16"></nldd-spacer>
            <nldd-button
              variant="primary"
              text="Naar het subsidieportaal"
              end-icon="arrow-right"
              href="/aanvrager/"
            ></nldd-button>
          </nldd-container>
        </nldd-card>
        <nldd-card accessible-label="Medewerkers van de Napp">
          <nldd-container padding="20">
            <nldd-icon name="shield-check-mark" size="32" color="lintblauw"></nldd-icon>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-title size="4"><h3>Medewerkers van de Napp</h3></nldd-title>
            <nldd-spacer size="8"></nldd-spacer>
            <nldd-rich-text>
              <p>
                Beoordelaars loggen in met SSO Rijk in de aparte
                beoordelingsomgeving. De wet berekent; u toetst en besluit.
              </p>
            </nldd-rich-text>
            <nldd-spacer size="16"></nldd-spacer>
            <nldd-button
              variant="secondary"
              text="Naar de beoordelingsomgeving"
              end-icon="arrow-right"
              href="/beoordelaar/"
            ></nldd-button>
          </nldd-container>
        </nldd-card>
        <nldd-card accessible-label="Openbaar register">
          <nldd-container padding="20">
            <nldd-icon name="books-vertical" size="32" color="lintblauw"></nldd-icon>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-title size="4"><h3>Iedereen</h3></nldd-title>
            <nldd-spacer size="8"></nldd-spacer>
            <nldd-rich-text>
              <p>
                Het openbare register toont alle bekendgemaakte subsidiebesluiten en
                statistieken over aanvragen en toekenningen. Geen login nodig.
              </p>
            </nldd-rich-text>
            <nldd-spacer size="16"></nldd-spacer>
            <nldd-button
              variant="secondary"
              text="Bekijk het register"
              end-icon="arrow-right"
              @click="router.push('/register')"
            ></nldd-button>
          </nldd-container>
        </nldd-card>
      </nldd-collection>
    </nldd-simple-section>

    <nldd-two-thirds-one-third-section>
      <div slot="left">
        <nldd-title size="2"><h2>De wet als uitvoerbare regels</h2></nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            Dit portaal draait op een machine-leesbare versie van de Wet op de
            politieke partijen. Het subsidiebesluit (artikel 15) is een beschikking
            in de zin van de Algemene wet bestuursrecht: de motiveringsplicht
            (AWB 3:46), de bezwaartermijn van zes weken (AWB 6:7) en de berekening
            van de termijn na bekendmaking (AWB 6:8) haken automatisch aan op elk
            besluit.
          </p>
          <p>
            De bedragen en staffels staan in een aparte ministeriële regeling. Zodra
            de definitieve wettekst beschikbaar is, wordt de wet vervangen zonder
            dat dit portaal verandert.
          </p>
        </nldd-rich-text>
      </div>
      <div slot="right">
        <nldd-card v-if="stats" accessible-label="Kerncijfers">
          <nldd-container padding="20" gap="8">
            <nldd-title size="5"><h3>Tot nu toe</h3></nldd-title>
            <nldd-list variant="simple">
              <nldd-list-item size="md">
                <nldd-text-cell text="Aanvragen" :supporting-text="String(stats.aantal_aanvragen)"></nldd-text-cell>
              </nldd-list-item>
              <nldd-list-item size="md">
                <nldd-text-cell text="Toegekend" :supporting-text="String(stats.aantal_toegekend)"></nldd-text-cell>
              </nldd-list-item>
              <nldd-list-item size="md">
                <nldd-text-cell text="Totaal toegekend bedrag" :supporting-text="euro(stats.totaal_toegekend_bedrag)"></nldd-text-cell>
              </nldd-list-item>
            </nldd-list>
          </nldd-container>
        </nldd-card>
      </div>
    </nldd-two-thirds-one-third-section>

    <nldd-page-footer>
      <nldd-container padding="24">
        <nldd-rich-text>
          <p>
            Demonstratie-omgeving van de Nederlandse autoriteit politieke partijen.
            Gebaseerd op het wetsvoorstel Wet op de politieke partijen (kamerstuk 36742);
            bedragen en voorwaarden zijn indicatief.
          </p>
        </nldd-rich-text>
      </nldd-container>
    </nldd-page-footer>
  </nldd-page>
</template>
