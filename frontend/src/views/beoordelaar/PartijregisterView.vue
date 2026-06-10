<script setup>
/**
 * Partijregister-beheer: zoeken en bladeren door alle geregistreerde
 * koppelingen tussen rechtspersoon (KvK) en aanduiding. Nieuwe koppelingen
 * ontstaan via een claim bij de eerste aanvraag, niet via handmatige
 * invoer. Beoordelaar-only; de backend handhaaft dat met 403.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session } from '../../session.js';

const router = useRouter();

const navItems = [
  { text: 'Werkvoorraad', to: '/' },
  { text: 'Partijregister', to: '/partijregister' },
  { text: "Scenario's", to: '/scenarios' },
];

const LIMIT = 25;
const zoek = ref('');
const pagina = ref(1);
const partijen = ref([]);
const totaal = ref(0);
const laden = ref(false);
const fout = ref('');
const melding = ref('');

const totaalPaginas = computed(() => Math.max(1, Math.ceil(totaal.value / LIMIT)));

const aantalFormat = new Intl.NumberFormat('nl-NL');

async function laad() {
  if (!session.beoordelaar) return;
  laden.value = true;
  fout.value = '';
  try {
    const resultaat = await api.beheerPartijen({
      zoek: zoek.value.trim(),
      offset: (pagina.value - 1) * LIMIT,
      limit: LIMIT,
    });
    partijen.value = resultaat.partijen;
    totaal.value = resultaat.totaal;
  } catch (e) {
    fout.value = e.message;
  } finally {
    laden.value = false;
  }
}

let debounceTimer = null;
function onZoek(event) {
  zoek.value = event.detail?.value ?? event.target?.value ?? '';
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    pagina.value = 1;
    laad();
  }, 300);
}
onBeforeUnmount(() => clearTimeout(debounceTimer));

function onPagina(event) {
  pagina.value = event.detail?.page ?? pagina.value;
  laad();
}

onMounted(laad);
watch(() => session.beoordelaar, laad);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Beoordelingsomgeving"
      :items="session.beoordelaar ? navItems : []"
      portal="beoordelaar"
    />

    <!-- Niet ingelogd -->
    <template v-if="session.loaded && !session.beoordelaar">
      <nldd-simple-section width="560px">
        <NBanner
          variant="warning"
          text="Inloggen vereist"
          supporting-text="Het partijregister is alleen toegankelijk voor beoordelaars van de Napp."
        />
        <nldd-spacer size="16"></nldd-spacer>
        <nldd-button
          variant="primary"
          text="Naar de inlogpagina"
          start-icon="login"
          @click="router.push('/')"
        ></nldd-button>
      </nldd-simple-section>
    </template>

    <template v-else-if="session.beoordelaar">
      <nldd-simple-section>
        <nldd-title size="2">
          <span slot="overline">Registratietaak van de Napp</span>
          <h2>Partijregister</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            De koppeling tussen rechtspersoon (KvK-nummer) en geregistreerde
            aanduiding, met het organisatiemodel. Nieuwe koppelingen ontstaan
            doordat een partij haar aanduiding claimt bij haar eerste
            aanvraag. De verkiezingsuitslagen zelf zijn referentiedata van de
            Kiesraad en zijn hier alleen te raadplegen.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-search-field
          :value="zoek"
          placeholder="Zoek op partijnaam of KvK-nummer"
          accessible-label="Zoek op partijnaam of KvK-nummer"
          @input="onZoek"
        ></nldd-search-field>
        <nldd-spacer size="16"></nldd-spacer>

        <NBanner v-if="melding" variant="success" :text="melding" />
        <NBanner v-if="fout" variant="critical" text="Laden mislukt" :supporting-text="fout" />
        <nldd-spacer v-if="melding || fout" size="16"></nldd-spacer>

        <template v-if="partijen.length">
          <nldd-table
            columns="minmax(240px,1fr) 110px 160px 110px 130px 110px"
            accessible-label="Geregistreerde partijen"
          >
            <nldd-table-row slot="header">
              <nldd-text-cell text="Naam"></nldd-text-cell>
              <nldd-text-cell text="KvK"></nldd-text-cell>
              <nldd-text-cell text="Organisatiemodel"></nldd-text-cell>
              <nldd-text-cell text="Kamerzetels" horizontal-alignment="right"></nldd-text-cell>
              <nldd-text-cell text="Uitslagen" horizontal-alignment="right"></nldd-text-cell>
              <nldd-text-cell text=""></nldd-text-cell>
            </nldd-table-row>
            <nldd-table-row v-for="partij in partijen" :key="partij.kvk_nummer">
              <nldd-text-cell :text="partij.naam" :query="zoek.trim()"></nldd-text-cell>
              <nldd-text-cell :text="partij.kvk_nummer" color="secondary"></nldd-text-cell>
              <nldd-cell>
                <nldd-tag
                  :color="partij.organisatiemodel === 'CENTRAAL' ? 'accent' : 'neutral'"
                  :text="partij.organisatiemodel === 'CENTRAAL' ? 'Centraal' : 'Decentraal'"
                ></nldd-tag>
              </nldd-cell>
              <nldd-text-cell
                :text="partij.kamerzetels ? String(partij.kamerzetels) : '–'"
                horizontal-alignment="right"
              ></nldd-text-cell>
              <nldd-text-cell
                :text="String(partij.aantal_uitslagen)"
                horizontal-alignment="right"
              ></nldd-text-cell>
              <nldd-cell horizontal-alignment="right">
                <nldd-button
                  variant="secondary"
                  size="sm"
                  text="Bekijk"
                  end-icon="chevron-right"
                  @click="router.push(`/partijregister/${partij.kvk_nummer}`)"
                ></nldd-button>
              </nldd-cell>
            </nldd-table-row>
          </nldd-table>
          <nldd-spacer size="16"></nldd-spacer>
          <nldd-pagination
            v-if="totaalPaginas > 1"
            :current="pagina"
            :total="totaalPaginas"
            centered
            @page-change="onPagina"
          ></nldd-pagination>
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-rich-text>
            <p>{{ aantalFormat.format(totaal) }} {{ totaal === 1 ? 'partij' : 'partijen' }} gevonden.</p>
          </nldd-rich-text>
        </template>
        <nldd-activity-indicator v-else-if="laden" show-text text="Register laden"></nldd-activity-indicator>
        <nldd-inline-dialog
          v-else
          icon="magnifier"
          text="Geen partijen gevonden"
          supporting-text="Pas de zoekopdracht aan. Nieuwe partijen verschijnen hier zodra hun claim bij de eerste aanvraag is bevestigd."
        ></nldd-inline-dialog>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
