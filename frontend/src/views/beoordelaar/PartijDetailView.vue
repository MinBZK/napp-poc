<script setup>
/**
 * Detail van een geregistreerde partij: gegevens bewerken, decentrale
 * uitslagen per orgaan inzien en corrigeren (regel toevoegen/verwijderen).
 */
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session } from '../../session.js';

const route = useRoute();
const router = useRouter();

const navItems = [
  { text: 'Werkvoorraad', to: '/' },
  { text: 'Partijregister', to: '/partijregister' },
  { text: "Scenario's", to: '/scenarios' },
];

const ORGAAN_LABELS = {
  GEMEENTERAAD: 'Gemeenteraad',
  PROVINCIALE_STATEN: 'Provinciale Staten',
  WATERSCHAP: 'Waterschap',
};

const kvk = computed(() => route.params.kvk);
const partij = ref(null);
const fout = ref('');
const melding = ref('');

const aantalFormat = new Intl.NumberFormat('nl-NL');

const uitslagenPerOrgaan = computed(() => {
  if (!partij.value) return [];
  return Object.keys(ORGAAN_LABELS)
    .map((orgaan) => ({
      orgaan,
      label: ORGAAN_LABELS[orgaan],
      uitslagen: partij.value.uitslagen.filter((u) => u.orgaan === orgaan),
    }))
    .filter((groep) => groep.uitslagen.length);
});

async function laad() {
  if (!session.beoordelaar) return;
  fout.value = '';
  try {
    partij.value = await api.beheerPartij(kvk.value);
  } catch (e) {
    fout.value = e.message;
  }
}

function veld(event) {
  return event.detail?.value ?? event.target?.value ?? '';
}

// --- Gegevens bewerken -------------------------------------------------------
const bewerkSheetEl = ref(null);
const bewerkOpen = ref(false);
const bewerk = ref({ naam: '', organisatiemodel: 'CENTRAAL', moederpartij_kvk: '' });
const bewerkFout = ref('');
const bewerkBezig = ref(false);

watch(bewerkOpen, async (open) => {
  if (!open) {
    bewerkSheetEl.value?.hide();
    return;
  }
  await nextTick();
  bewerkSheetEl.value?.show();
});

function openBewerken() {
  bewerk.value = {
    naam: partij.value.naam,
    organisatiemodel: partij.value.organisatiemodel,
    moederpartij_kvk: partij.value.moederpartij_kvk ?? '',
  };
  bewerkFout.value = '';
  bewerkOpen.value = true;
}

async function bewaarGegevens() {
  bewerkFout.value = '';
  bewerkBezig.value = true;
  try {
    partij.value = await api.beheerPartijWijzigen(kvk.value, {
      naam: bewerk.value.naam.trim(),
      organisatiemodel: bewerk.value.organisatiemodel,
      moederpartij_kvk: bewerk.value.moederpartij_kvk.trim() || null,
    });
    bewerkOpen.value = false;
    melding.value = 'De partijgegevens zijn gewijzigd.';
  } catch (e) {
    bewerkFout.value = e.message;
  } finally {
    bewerkBezig.value = false;
  }
}

// --- Uitslag toevoegen -------------------------------------------------------
const uitslagSheetEl = ref(null);
const uitslagOpen = ref(false);
const nieuweUitslag = ref({ orgaan: 'GEMEENTERAAD', gebied_code: '', zetels: 1 });
const gebieden = ref([]);
const uitslagFout = ref('');
const uitslagBezig = ref(false);

watch(uitslagOpen, async (open) => {
  if (!open) {
    uitslagSheetEl.value?.hide();
    return;
  }
  await nextTick();
  uitslagSheetEl.value?.show();
});

async function laadGebieden() {
  try {
    gebieden.value = await api.beheerGebieden(nieuweUitslag.value.orgaan);
    if (!gebieden.value.some((g) => g.code === nieuweUitslag.value.gebied_code)) {
      nieuweUitslag.value.gebied_code = gebieden.value[0]?.code ?? '';
    }
  } catch (e) {
    uitslagFout.value = e.message;
  }
}

async function openUitslagToevoegen() {
  nieuweUitslag.value = { orgaan: 'GEMEENTERAAD', gebied_code: '', zetels: 1 };
  uitslagFout.value = '';
  uitslagOpen.value = true;
  await laadGebieden();
}

async function wisselOrgaan(event) {
  nieuweUitslag.value.orgaan = veld(event);
  await laadGebieden();
}

async function voegUitslagToe() {
  uitslagFout.value = '';
  uitslagBezig.value = true;
  try {
    partij.value = await api.beheerUitslagToevoegen(kvk.value, {
      orgaan: nieuweUitslag.value.orgaan,
      gebied_code: nieuweUitslag.value.gebied_code,
      zetels: Number(nieuweUitslag.value.zetels),
    });
    uitslagOpen.value = false;
    melding.value = 'De uitslag is toegevoegd.';
  } catch (e) {
    uitslagFout.value = e.message;
  } finally {
    uitslagBezig.value = false;
  }
}

// --- Uitslag verwijderen -----------------------------------------------------
const verwijderModalEl = ref(null);
const verwijderKandidaat = ref(null);

watch(verwijderKandidaat, async (kandidaat) => {
  if (!kandidaat) {
    verwijderModalEl.value?.hide();
    return;
  }
  await nextTick();
  verwijderModalEl.value?.show();
});

async function verwijderUitslag() {
  const kandidaat = verwijderKandidaat.value;
  if (!kandidaat) return;
  try {
    partij.value = await api.beheerUitslagVerwijderen(
      kvk.value,
      kandidaat.orgaan,
      kandidaat.gebied_code,
    );
    melding.value = 'De uitslag is verwijderd.';
  } catch (e) {
    fout.value = e.message;
  }
  verwijderKandidaat.value = null;
}

onMounted(laad);
watch(() => session.beoordelaar, laad);
watch(kvk, laad);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Beoordelingsomgeving"
      :items="session.beoordelaar ? navItems : []"
      portal="beoordelaar"
    />

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
        <nldd-button
          variant="neutral-transparent"
          size="sm"
          text="Terug naar het partijregister"
          start-icon="chevron-left"
          @click="router.push('/partijregister')"
        ></nldd-button>
        <nldd-spacer size="16"></nldd-spacer>

        <NBanner v-if="fout" variant="critical" text="Er ging iets mis" :supporting-text="fout" />

        <template v-if="partij">
          <nldd-title size="2">
            <span slot="overline">KvK-nummer {{ partij.kvk_nummer }}</span>
            <h2>{{ partij.naam }}</h2>
            <div slot="actions">
              <nldd-button-group orientation="horizontal">
                <nldd-button
                  variant="secondary"
                  text="Gegevens bewerken"
                  start-icon="pencil"
                  @click="openBewerken"
                ></nldd-button>
                <nldd-button
                  variant="primary"
                  text="Uitslag toevoegen"
                  start-icon="plus"
                  @click="openUitslagToevoegen"
                ></nldd-button>
              </nldd-button-group>
            </div>
          </nldd-title>
          <nldd-spacer size="12"></nldd-spacer>

          <nldd-container layout="wrap" gap="8">
            <nldd-tag
              :color="partij.organisatiemodel === 'CENTRAAL' ? 'accent' : 'neutral'"
              :text="partij.organisatiemodel === 'CENTRAAL' ? 'Centraal georganiseerd' : 'Decentraal georganiseerd'"
            ></nldd-tag>
            <nldd-tag
              v-if="partij.kamerzetels"
              color="success"
              :text="`${partij.kamerzetels} kamerzetels`"
            ></nldd-tag>
            <nldd-tag
              v-if="partij.moederpartij_kvk"
              color="neutral"
              :text="`Afdeling van ${partij.moederpartij_naam ?? partij.moederpartij_kvk}`"
            ></nldd-tag>
          </nldd-container>
          <nldd-spacer size="8"></nldd-spacer>

          <template v-if="melding">
            <NBanner variant="success" :text="melding" />
            <nldd-spacer size="8"></nldd-spacer>
          </template>

          <template v-if="partij.moederpartij_kvk">
            <nldd-link
              size="sm"
              end-icon="chevron-right"
              :href="`/beoordelaar/partijregister/${partij.moederpartij_kvk}`"
              :text="`Naar moederpartij ${partij.moederpartij_naam ?? partij.moederpartij_kvk}`"
              @click.prevent="router.push(`/partijregister/${partij.moederpartij_kvk}`)"
            ></nldd-link>
            <nldd-spacer size="8"></nldd-spacer>
          </template>

          <nldd-spacer size="16"></nldd-spacer>

          <template v-if="uitslagenPerOrgaan.length">
            <template v-for="groep in uitslagenPerOrgaan" :key="groep.orgaan">
              <nldd-title size="4"><h3>{{ groep.label }}</h3></nldd-title>
              <nldd-spacer size="8"></nldd-spacer>
              <nldd-table
                columns="minmax(200px,1fr) 130px 140px 100px 150px"
                :accessible-label="`Uitslagen ${groep.label}`"
              >
                <nldd-table-row slot="header">
                  <nldd-text-cell text="Gebied"></nldd-text-cell>
                  <nldd-text-cell text="Code"></nldd-text-cell>
                  <nldd-text-cell text="Inwoneraantal" horizontal-alignment="right"></nldd-text-cell>
                  <nldd-text-cell text="Zetels" horizontal-alignment="right"></nldd-text-cell>
                  <nldd-text-cell text=""></nldd-text-cell>
                </nldd-table-row>
                <nldd-table-row
                  v-for="uitslag in groep.uitslagen"
                  :key="`${uitslag.orgaan}:${uitslag.gebied_code}`"
                >
                  <nldd-text-cell :text="uitslag.gebied_naam ?? uitslag.gebied_code"></nldd-text-cell>
                  <nldd-text-cell :text="uitslag.gebied_code" color="secondary"></nldd-text-cell>
                  <nldd-text-cell
                    :text="uitslag.inwoneraantal ? aantalFormat.format(uitslag.inwoneraantal) : '–'"
                    horizontal-alignment="right"
                  ></nldd-text-cell>
                  <nldd-text-cell
                    :text="String(uitslag.zetels)"
                    horizontal-alignment="right"
                  ></nldd-text-cell>
                  <nldd-cell horizontal-alignment="right">
                    <nldd-button
                      variant="critical-transparent"
                      size="sm"
                      text="Verwijderen"
                      start-icon="trash"
                      @click="verwijderKandidaat = uitslag"
                    ></nldd-button>
                  </nldd-cell>
                </nldd-table-row>
              </nldd-table>
              <nldd-spacer size="24"></nldd-spacer>
            </template>
          </template>
          <nldd-inline-dialog
            v-else
            icon="inbox"
            text="Geen decentrale uitslagen geregistreerd"
            supporting-text="Voeg een uitslag toe wanneer deze partij een zetel haalde in een gemeenteraad, provinciale staten of waterschap."
          ></nldd-inline-dialog>
        </template>
        <nldd-activity-indicator v-else-if="!fout" show-text text="Partij laden"></nldd-activity-indicator>
      </nldd-simple-section>

      <!-- Gegevens bewerken -->
      <nldd-sheet
        ref="bewerkSheetEl"
        placement="right"
        width="480px"
        accessible-label="Partijgegevens bewerken"
        @close="bewerkOpen = false"
      >
        <nldd-container padding="24" gap="16">
          <nldd-title size="3">
            <span slot="overline">KvK-nummer {{ kvk }}</span>
            <h3>Gegevens bewerken</h3>
          </nldd-title>
          <nldd-form novalidate @submit.prevent="bewaarGegevens">
            <nldd-form-field label="Geregistreerde aanduiding">
              <nldd-text-field
                :value="bewerk.naam"
                name="naam"
                @input="bewerk.naam = veld($event)"
              ></nldd-text-field>
            </nldd-form-field>
            <nldd-form-field label="Organisatiemodel">
              <nldd-dropdown>
                <select
                  :value="bewerk.organisatiemodel"
                  @change="bewerk.organisatiemodel = veld($event)"
                >
                  <option value="CENTRAAL">Centraal (afdelingen onder één KvK)</option>
                  <option value="DECENTRAAL">Decentraal (afdelingen als eigen rechtspersoon)</option>
                </select>
              </nldd-dropdown>
            </nldd-form-field>
            <nldd-form-field label="Moederpartij (optioneel)">
              <nldd-text-field
                :value="bewerk.moederpartij_kvk"
                name="moederpartij_kvk"
                placeholder="KvK-nummer van de moederpartij"
                @input="bewerk.moederpartij_kvk = veld($event)"
              ></nldd-text-field>
            </nldd-form-field>
            <template v-if="bewerkFout">
              <NBanner variant="critical" :text="bewerkFout" />
            </template>
            <nldd-form-actions>
              <nldd-button
                variant="primary"
                type="submit"
                text="Opslaan"
                :disabled="bewerkBezig || undefined"
              ></nldd-button>
              <nldd-button
                variant="secondary"
                text="Annuleren"
                @click="bewerkOpen = false"
              ></nldd-button>
            </nldd-form-actions>
          </nldd-form>
        </nldd-container>
      </nldd-sheet>

      <!-- Uitslag toevoegen -->
      <nldd-sheet
        ref="uitslagSheetEl"
        placement="right"
        width="480px"
        accessible-label="Uitslag toevoegen"
        @close="uitslagOpen = false"
      >
        <nldd-container padding="24" gap="16">
          <nldd-title size="3">
            <span slot="overline">{{ partij?.naam }}</span>
            <h3>Uitslag toevoegen</h3>
          </nldd-title>
          <nldd-form novalidate @submit.prevent="voegUitslagToe">
            <nldd-form-field label="Orgaan">
              <nldd-dropdown>
                <select :value="nieuweUitslag.orgaan" @change="wisselOrgaan">
                  <option
                    v-for="(label, orgaan) in ORGAAN_LABELS"
                    :key="orgaan"
                    :value="orgaan"
                  >
                    {{ label }}
                  </option>
                </select>
              </nldd-dropdown>
            </nldd-form-field>
            <nldd-form-field label="Gebied">
              <nldd-dropdown>
                <select
                  :value="nieuweUitslag.gebied_code"
                  @change="nieuweUitslag.gebied_code = veld($event)"
                >
                  <option v-for="gebied in gebieden" :key="gebied.code" :value="gebied.code">
                    {{ gebied.naam }} ({{ gebied.code }})
                  </option>
                </select>
              </nldd-dropdown>
            </nldd-form-field>
            <nldd-form-field label="Zetels">
              <nldd-number-field
                :value="nieuweUitslag.zetels"
                name="zetels"
                min="1"
                step="1"
                @input="nieuweUitslag.zetels = veld($event)"
                @change="nieuweUitslag.zetels = veld($event)"
              ></nldd-number-field>
            </nldd-form-field>
            <template v-if="uitslagFout">
              <NBanner variant="critical" :text="uitslagFout" />
            </template>
            <nldd-form-actions>
              <nldd-button
                variant="primary"
                type="submit"
                text="Toevoegen"
                :disabled="uitslagBezig || undefined"
              ></nldd-button>
              <nldd-button
                variant="secondary"
                text="Annuleren"
                @click="uitslagOpen = false"
              ></nldd-button>
            </nldd-form-actions>
          </nldd-form>
        </nldd-container>
      </nldd-sheet>

      <!-- Verwijderen bevestigen -->
      <nldd-modal-dialog
        ref="verwijderModalEl"
        variant="alert"
        text="Uitslag verwijderen?"
        :supporting-text="verwijderKandidaat
          ? `De uitslag voor ${verwijderKandidaat.gebied_naam ?? verwijderKandidaat.gebied_code} (${ORGAAN_LABELS[verwijderKandidaat.orgaan]}) wordt uit het register verwijderd.`
          : ''"
        accessible-label="Uitslag verwijderen"
        @close="verwijderKandidaat = null"
      >
        <nldd-button
          slot="actions"
          variant="destructive"
          text="Verwijderen"
          @click="verwijderUitslag"
        ></nldd-button>
        <nldd-button
          slot="actions"
          variant="secondary"
          text="Annuleren"
          @click="verwijderKandidaat = null"
        ></nldd-button>
      </nldd-modal-dialog>
    </template>
  </nldd-page>
</template>
