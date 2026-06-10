<script setup>
/**
 * Partijregister-beheer: zoeken en bladeren door alle geregistreerde
 * partijen, met een registratieformulier voor nieuwe partijen.
 * Beoordelaar-only; de backend handhaaft dat met 403.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue';
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

// --- Nieuw-registreren-sheet -----------------------------------------------
const sheetEl = ref(null);
const sheetOpen = ref(false);
const nieuw = ref({ kvk_nummer: '', naam: '', organisatiemodel: 'CENTRAAL', moederpartij_kvk: '' });
const nieuwFout = ref('');
const bezig = ref(false);

watch(sheetOpen, async (open) => {
  if (!open) {
    sheetEl.value?.hide();
    return;
  }
  await nextTick();
  sheetEl.value?.show();
});

function openNieuw() {
  nieuw.value = { kvk_nummer: '', naam: '', organisatiemodel: 'CENTRAAL', moederpartij_kvk: '' };
  nieuwFout.value = '';
  sheetOpen.value = true;
}

function veld(event) {
  return event.detail?.value ?? event.target?.value ?? '';
}

async function registreer() {
  nieuwFout.value = '';
  bezig.value = true;
  try {
    const detail = await api.beheerPartijRegistreren({
      kvk_nummer: nieuw.value.kvk_nummer.trim(),
      naam: nieuw.value.naam.trim(),
      organisatiemodel: nieuw.value.organisatiemodel,
      moederpartij_kvk: nieuw.value.moederpartij_kvk.trim() || null,
    });
    sheetOpen.value = false;
    melding.value = `Partij '${detail.naam}' is geregistreerd onder KvK-nummer ${detail.kvk_nummer}.`;
    await laad();
  } catch (e) {
    nieuwFout.value = e.message;
  } finally {
    bezig.value = false;
  }
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
          <div slot="actions">
            <nldd-button
              variant="primary"
              text="Partij registreren"
              start-icon="plus"
              @click="openNieuw"
            ></nldd-button>
          </div>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            De koppeling tussen rechtspersoon (KvK-nummer) en geregistreerde
            aanduiding, met organisatiemodel en decentrale verkiezingsuitslagen.
            Dit register bepaalt welke aanspraken een partij in het
            aanvraagportaal ziet.
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
          supporting-text="Pas de zoekopdracht aan of registreer een nieuwe partij."
        ></nldd-inline-dialog>
      </nldd-simple-section>

      <!-- Registratieformulier -->
      <nldd-sheet
        ref="sheetEl"
        placement="right"
        width="480px"
        accessible-label="Partij registreren"
        @close="sheetOpen = false"
      >
        <nldd-container padding="24" gap="16">
          <nldd-title size="3">
            <span slot="overline">Partijregister</span>
            <h3>Partij registreren</h3>
          </nldd-title>
          <nldd-form novalidate @submit.prevent="registreer">
            <nldd-form-field label="KvK-nummer">
              <nldd-text-field
                :value="nieuw.kvk_nummer"
                name="kvk_nummer"
                placeholder="8 cijfers"
                @input="nieuw.kvk_nummer = veld($event)"
              ></nldd-text-field>
              <nldd-form-field-help-text>
                Het KvK-nummer van de rechtspersoon die de aanduiding voert.
              </nldd-form-field-help-text>
            </nldd-form-field>
            <nldd-form-field label="Geregistreerde aanduiding">
              <nldd-text-field
                :value="nieuw.naam"
                name="naam"
                placeholder="Bijvoorbeeld: Partij voor de Toekomst"
                @input="nieuw.naam = veld($event)"
              ></nldd-text-field>
            </nldd-form-field>
            <nldd-form-field label="Organisatiemodel">
              <nldd-dropdown>
                <select
                  :value="nieuw.organisatiemodel"
                  @change="nieuw.organisatiemodel = veld($event)"
                >
                  <option value="CENTRAAL">Centraal (afdelingen onder één KvK)</option>
                  <option value="DECENTRAAL">Decentraal (afdelingen als eigen rechtspersoon)</option>
                </select>
              </nldd-dropdown>
            </nldd-form-field>
            <nldd-form-field label="Moederpartij (optioneel)">
              <nldd-text-field
                :value="nieuw.moederpartij_kvk"
                name="moederpartij_kvk"
                placeholder="KvK-nummer van de moederpartij"
                @input="nieuw.moederpartij_kvk = veld($event)"
              ></nldd-text-field>
              <nldd-form-field-help-text>
                Alleen voor afdelingen met een eigen rechtspersoon.
              </nldd-form-field-help-text>
            </nldd-form-field>
            <template v-if="nieuwFout">
              <NBanner variant="critical" :text="nieuwFout" />
            </template>
            <nldd-form-actions>
              <nldd-button
                variant="primary"
                type="submit"
                text="Registreren"
                :disabled="bezig || undefined"
              ></nldd-button>
              <nldd-button
                variant="secondary"
                text="Annuleren"
                @click="sheetOpen = false"
              ></nldd-button>
            </nldd-form-actions>
          </nldd-form>
        </nldd-container>
      </nldd-sheet>
    </template>
  </nldd-page>
</template>
