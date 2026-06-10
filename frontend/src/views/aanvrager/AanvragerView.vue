<script setup>
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session, refreshSession } from '../../session.js';
import { euro, datum, onderdelen, statusLabel, statusColor } from '../../format.js';

const router = useRouter();

const kvk = ref('');
const loginFout = ref('');
const bezig = ref(false);

// Two-step login: after the KvK number the mocked eHerkenning may offer
// multiple representation profiles (full board vs. branch volmacht).
const stap = ref('kvk'); // 'kvk' | 'machtiging'
const partijNaam = ref('');
const profielen = ref([]);
const afdelingKiezen = ref(false);
const zoek = ref('');

const aanvragen = ref([]);
const laden = ref(false);
const demoVoorbeelden = ref([]);

const demoTekst =
  'eHerkenning is gesimuleerd: alleen het KVK-nummer telt. Kies hieronder een ' +
  'voorbeeldpartij, of voer elk ander nummer in om als ongeregistreerde ' +
  'organisatie in te loggen.';

const ORGAAN_LABEL = {
  GEMEENTERAAD: 'Gemeenteraad',
  PROVINCIALE_STATEN: 'Provinciale staten',
  WATERSCHAP: 'Waterschap',
};

function kiesDemo(d) {
  kvk.value = d.kvk_nummer;
  login();
}

const navItems = computed(() =>
  session.aanvrager
    ? [
        { text: 'Mijn aanvragen', to: '/' },
        { text: 'Nieuwe aanvraag', to: '/nieuw' },
      ]
    : [],
);

const beperkteProfielen = computed(() =>
  profielen.value.filter((p) => p.type === 'BEPERKT'),
);

const aanvragerOverline = computed(() => {
  const a = session.aanvrager;
  if (!a) return '';
  const afdeling =
    a.machtiging?.type === 'BEPERKT' ? ` · afdeling ${a.machtiging.gebied_naam}` : '';
  return `${a.partij_naam}${afdeling} · KVK ${a.kvk_nummer}`;
});

const MAX_GETOOND = 12;
const gefilterdeProfielen = computed(() => {
  const q = zoek.value.trim().toLowerCase();
  return q
    ? beperkteProfielen.value.filter((p) =>
        p.gebied_naam.toLowerCase().includes(q),
      )
    : beperkteProfielen.value;
});
const getoondeProfielen = computed(() =>
  gefilterdeProfielen.value.slice(0, MAX_GETOOND),
);
const restAantal = computed(
  () => gefilterdeProfielen.value.length - getoondeProfielen.value.length,
);

async function login() {
  loginFout.value = '';
  if (!/^\d{8}$/.test(kvk.value.trim())) {
    loginFout.value = 'Vul een geldig KVK-nummer in (8 cijfers).';
    return;
  }
  bezig.value = true;
  try {
    const result = await api.eherkenningMachtigingen(kvk.value.trim());
    profielen.value = result.profielen ?? [];
    partijNaam.value = result.partij_naam ?? `Organisatie ${kvk.value.trim()}`;
    if (beperkteProfielen.value.length) {
      // The party knows branch volmachten: ask on whose behalf to log in.
      stap.value = 'machtiging';
      afdelingKiezen.value = false;
      zoek.value = '';
    } else {
      await doLogin(null);
    }
  } catch (e) {
    loginFout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

async function doLogin(machtiging) {
  loginFout.value = '';
  bezig.value = true;
  try {
    await api.eherkenningLogin(kvk.value.trim(), machtiging);
    await refreshSession();
    stap.value = 'kvk';
    profielen.value = [];
  } catch (e) {
    loginFout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

function terugNaarKvk() {
  stap.value = 'kvk';
  profielen.value = [];
  afdelingKiezen.value = false;
  zoek.value = '';
  loginFout.value = '';
}

async function laadAanvragen() {
  if (!session.aanvrager) return;
  laden.value = true;
  try {
    aanvragen.value = await api.mijnAanvragen();
  } finally {
    laden.value = false;
  }
}

// --- Rekening voor uitbetaling (één rekening per rechtspersoon, art. 27) ---
const rekening = ref(null);
const rekeningSheetEl = ref(null);
const rekeningOpen = ref(false);
const rekeningForm = ref({ iban: '', tenaamstelling: '' });
const rekeningFout = ref('');
const rekeningBezig = ref(false);
const rekeningMelding = ref('');

// Only the signing-authorized board (VOLLEDIG) manages the account; a
// branch volmacht sees it read-only.
const beperkteMachtiging = computed(
  () => session.aanvrager?.machtiging?.type === 'BEPERKT',
);

function veld(event) {
  return event.detail?.value ?? event.target?.value ?? '';
}

watch(rekeningOpen, async (open) => {
  if (!open) {
    rekeningSheetEl.value?.hide();
    return;
  }
  await nextTick();
  rekeningSheetEl.value?.show();
});

function openRekening() {
  rekeningForm.value = {
    iban: rekening.value?.iban ?? '',
    tenaamstelling: rekening.value?.tenaamstelling ?? '',
  };
  rekeningFout.value = '';
  rekeningMelding.value = '';
  rekeningOpen.value = true;
}

async function bewaarRekening() {
  rekeningFout.value = '';
  rekeningBezig.value = true;
  try {
    rekening.value = await api.mijnRekeningWijzigen({
      iban: rekeningForm.value.iban.trim(),
      tenaamstelling: rekeningForm.value.tenaamstelling.trim(),
    });
    rekeningOpen.value = false;
    rekeningMelding.value = 'Het rekeningnummer voor uitbetaling is vastgelegd.';
  } catch (e) {
    rekeningFout.value = e.message;
  } finally {
    rekeningBezig.value = false;
  }
}

async function laadRekening() {
  if (!session.aanvrager) {
    rekening.value = null;
    return;
  }
  try {
    rekening.value = await api.mijnRekening();
  } catch {
    rekening.value = null;
  }
}

onMounted(async () => {
  laadAanvragen();
  laadRekening();
  try {
    demoVoorbeelden.value = await api.registerDemo();
  } catch {
    demoVoorbeelden.value = [];
  }
});
watch(
  () => session.aanvrager,
  () => {
    laadAanvragen();
    laadRekening();
  },
);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Subsidieportaal politieke partijen"
      :items="navItems"
      portal="aanvrager"
    />

    <!-- Niet ingelogd: eHerkenning (gemockt) -->
    <template v-if="session.loaded && !session.aanvrager && stap === 'machtiging'">
      <nldd-simple-section width="560px">
        <nldd-title size="2">
          <span slot="overline">{{ partijNaam }} · KVK {{ kvk }}</span>
          <h2>Namens wie logt u in?</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            Deze partij is centraal georganiseerd: afdelingen vallen onder de
            landelijke rechtspersoon. Afdelingsbestuurders kunnen met een
            beperkte volmacht namens de partij een deelaanvraag doen.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-list variant="box">
          <nldd-list-item size="md" type="button" @click="doLogin({ type: 'VOLLEDIG' })">
            <nldd-title-cell
              text="De gehele partij"
              supporting-text="Tekenbevoegd bestuur: alle aanspraken, landelijk en decentraal"
            ></nldd-title-cell>
            <nldd-spacer-cell size="8"></nldd-spacer-cell>
            <nldd-icon-cell icon="chevron-right" size="16"></nldd-icon-cell>
          </nldd-list-item>
          <nldd-list-item size="md" type="button" @click="afdelingKiezen = !afdelingKiezen">
            <nldd-title-cell
              text="Een afdeling"
              :supporting-text="`Beperkte machtiging voor één gebied (${beperkteProfielen.length} gebieden)`"
            ></nldd-title-cell>
            <nldd-spacer-cell size="8"></nldd-spacer-cell>
            <nldd-icon-cell
              :icon="afdelingKiezen ? 'chevron-up' : 'chevron-down'"
              size="16"
            ></nldd-icon-cell>
          </nldd-list-item>
        </nldd-list>

        <template v-if="afdelingKiezen">
          <nldd-spacer size="16"></nldd-spacer>
          <nldd-form-field label="Zoek uw gebied">
            <nldd-text-field
              name="zoek-gebied"
              :value="zoek"
              placeholder="Bijvoorbeeld: Utrecht"
              @input="zoek = $event.detail?.value ?? $event.target?.value ?? ''"
            ></nldd-text-field>
          </nldd-form-field>
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-list v-if="getoondeProfielen.length" variant="box">
            <nldd-list-item
              v-for="p in getoondeProfielen"
              :key="`${p.orgaan}:${p.gebied_code}`"
              size="sm"
              type="button"
              @click="doLogin({ type: 'BEPERKT', gebied_code: p.gebied_code })"
            >
              <nldd-text-cell
                :text="p.gebied_naam"
                :supporting-text="ORGAAN_LABEL[p.orgaan] ?? p.orgaan"
              ></nldd-text-cell>
              <nldd-spacer-cell size="8"></nldd-spacer-cell>
              <nldd-icon-cell icon="chevron-right" size="16"></nldd-icon-cell>
            </nldd-list-item>
          </nldd-list>
          <nldd-rich-text v-if="restAantal > 0">
            <p>Nog {{ restAantal }} gebieden. Verfijn uw zoekopdracht.</p>
          </nldd-rich-text>
          <nldd-rich-text v-if="!getoondeProfielen.length">
            <p>Geen gebied gevonden voor &lsquo;{{ zoek }}&rsquo;.</p>
          </nldd-rich-text>
        </template>

        <nldd-spacer size="24"></nldd-spacer>
        <nldd-button
          variant="neutral-transparent"
          text="Terug"
          start-icon="arrow-left"
          :disabled="bezig || undefined"
          @click="terugNaarKvk"
        ></nldd-button>

        <template v-if="loginFout">
          <nldd-spacer size="16"></nldd-spacer>
          <NBanner variant="critical" :text="loginFout" />
        </template>
      </nldd-simple-section>
    </template>

    <template v-else-if="session.loaded && !session.aanvrager">
      <nldd-simple-section width="560px">
        <nldd-title size="2">
          <span slot="overline">Voor politieke partijen</span>
          <h2>Inloggen met eHerkenning</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            Politieke partijen zijn verenigingen of stichtingen en loggen in met
            eHerkenning namens hun organisatie.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="16"></nldd-spacer>
        <NBanner
          variant="warning"
          text="Demo-omgeving"
          :supporting-text="demoTekst"
        />
        <nldd-spacer size="24"></nldd-spacer>
        <nldd-card accessible-label="eHerkenning-login">
          <nldd-container padding="24">
            <nldd-form novalidate @submit.prevent="login">
              <nldd-form-field label="KVK-nummer">
                <nldd-text-field
                  name="kvk"
                  :value="kvk"
                  placeholder="12345678"
                  :invalid="loginFout.includes('KVK') || undefined"
                  error-message="kvk-fout"
                  @input="kvk = $event.detail?.value ?? $event.target?.value ?? ''"
                ></nldd-text-field>
                <nldd-form-field-error-text id="kvk-fout">
                  Vul een geldig KVK-nummer in (8 cijfers).
                </nldd-form-field-error-text>
              </nldd-form-field>
              <nldd-form-actions>
                <nldd-button
                  variant="primary"
                  type="submit"
                  text="Inloggen met eHerkenning"
                  start-icon="lock-closed"
                  :disabled="bezig || undefined"
                ></nldd-button>
              </nldd-form-actions>
            </nldd-form>
            <template v-if="loginFout">
              <nldd-spacer size="16"></nldd-spacer>
              <NBanner variant="critical" :text="loginFout" />
            </template>
          </nldd-container>
        </nldd-card>

        <template v-if="demoVoorbeelden.length">
          <nldd-spacer size="32"></nldd-spacer>
          <nldd-title size="4"><h3>Voorbeeldpartijen (demo)</h3></nldd-title>
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-list variant="box">
            <nldd-list-item
              v-for="d in demoVoorbeelden"
              :key="d.kvk_nummer"
              size="sm"
              type="button"
              @click="kiesDemo(d)"
            >
              <nldd-text-cell
                :text="d.naam"
                :supporting-text="d.profiel"
              ></nldd-text-cell>
              <nldd-text-cell
                :text="d.kvk_nummer"
                color="secondary"
                horizontal-alignment="right"
              ></nldd-text-cell>
              <nldd-spacer-cell size="8"></nldd-spacer-cell>
              <nldd-icon-cell icon="chevron-right" size="16"></nldd-icon-cell>
            </nldd-list-item>
          </nldd-list>
        </template>
      </nldd-simple-section>
    </template>

    <!-- Ingelogd: eigen aanvragen -->
    <template v-else-if="session.aanvrager">
      <nldd-simple-section>
        <nldd-title size="2">
          <span slot="overline">{{ aanvragerOverline }}</span>
          <h2>Uw subsidieaanvragen</h2>
          <div v-if="aanvragen.length" slot="actions">
            <nldd-button
              variant="primary"
              text="Nieuwe aanvraag"
              start-icon="plus"
              @click="router.push('/nieuw')"
            ></nldd-button>
          </div>
        </nldd-title>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-list v-if="aanvragen.length" variant="box">
          <nldd-list-item
            v-for="item in aanvragen"
            :key="item.aanvraag.id"
            size="md"
            type="button"
            @click="router.push(`/aanvraag/${item.aanvraag.id}`)"
          >
            <nldd-title-cell
              :text="`Jaaraanvraag ${item.aanvraag.subsidiejaar}`"
              :supporting-text="`${onderdelen(item.aanvraag.componenten.length)} · ingediend op ${datum(item.aanvraag.aanvraag_datum)}`"
            ></nldd-title-cell>
            <nldd-text-cell
              v-if="item.besluit"
              width="fit-content"
              :text="euro(item.besluit.subsidiebedrag)"
              horizontal-alignment="right"
            ></nldd-text-cell>
            <nldd-spacer-cell size="12"></nldd-spacer-cell>
            <nldd-cell width="fit-content">
              <nldd-tag
                :color="statusColor(item.aanvraag.status, item.besluit)"
                :text="statusLabel(item.aanvraag.status, item.besluit)"
              ></nldd-tag>
            </nldd-cell>
            <nldd-spacer-cell size="8"></nldd-spacer-cell>
            <nldd-icon-cell icon="chevron-right" size="16" color="secondary"></nldd-icon-cell>
          </nldd-list-item>
        </nldd-list>

        <nldd-inline-dialog
          v-else-if="!laden"
          icon="inbox"
          text="Nog geen aanvragen"
          supporting-text="Dien uw eerste subsidieaanvraag in. U ziet direct of uw partij aan de voorwaarden voldoet."
        >
          <nldd-button
            slot="actions"
            variant="primary"
            text="Nieuwe aanvraag"
            @click="router.push('/nieuw')"
          ></nldd-button>
        </nldd-inline-dialog>

        <!-- Rekening voor uitbetaling: één per rechtspersoon (art. 27 Wpp) -->
        <nldd-spacer size="40"></nldd-spacer>
        <nldd-title size="3">
          <h3>Rekening voor uitbetaling</h3>
          <div
            v-if="rekening?.in_register && !beperkteMachtiging"
            slot="actions"
          >
            <nldd-button
              variant="secondary"
              :text="rekening?.iban ? 'Rekening wijzigen' : 'Rekening opgeven'"
              start-icon="pencil"
              @click="openRekening"
            ></nldd-button>
          </div>
        </nldd-title>
        <nldd-spacer size="8"></nldd-spacer>
        <nldd-rich-text>
          <p>
            De subsidie wordt verstrekt aan de rechtspersoon (artikel 27 Wpp):
            er geldt één rekeningnummer per partij, op naam van de
            rechtspersoon.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="12"></nldd-spacer>

        <template v-if="rekeningMelding">
          <NBanner variant="success" :text="rekeningMelding" />
          <nldd-spacer size="12"></nldd-spacer>
        </template>

        <nldd-list variant="box">
          <nldd-list-item size="md">
            <nldd-title-cell
              :text="rekening?.iban ?? 'Nog niet opgegeven'"
              :supporting-text="
                rekening?.iban
                  ? `Tenaamstelling: ${rekening.tenaamstelling}`
                  : 'Voor uitbetaling van het voorschot is een rekeningnummer op naam van de rechtspersoon nodig.'
              "
            ></nldd-title-cell>
            <template v-if="rekening && !rekening.iban">
              <nldd-spacer-cell size="12"></nldd-spacer-cell>
              <nldd-cell width="fit-content">
                <nldd-tag color="warning" text="Ontbreekt"></nldd-tag>
              </nldd-cell>
            </template>
          </nldd-list-item>
        </nldd-list>

        <template v-if="beperkteMachtiging">
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-rich-text>
            <p>
              Alleen het tekenbevoegd bestuur van de partij kan het
              rekeningnummer opgeven of wijzigen. Met uw beperkte machtiging
              als afdelingsbestuurder kunt u het hier alleen inzien.
            </p>
          </nldd-rich-text>
        </template>
        <template v-else-if="rekening && !rekening.in_register">
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-rich-text>
            <p>
              Uw organisatie staat nog niet in het partijregister van de Napp.
              Zodra de registratie is vastgelegd kan het bestuur hier een
              rekeningnummer opgeven.
            </p>
          </nldd-rich-text>
        </template>
      </nldd-simple-section>

      <!-- Rekening opgeven of wijzigen -->
      <nldd-sheet
        ref="rekeningSheetEl"
        placement="right"
        width="480px"
        accessible-label="Rekening voor uitbetaling"
        @close="rekeningOpen = false"
      >
        <nldd-container padding="24" gap="16">
          <nldd-title size="3">
            <span slot="overline">{{ session.aanvrager?.partij_naam }}</span>
            <h3>Rekening voor uitbetaling</h3>
          </nldd-title>
          <nldd-rich-text>
            <p>
              Het IBAN wordt gevalideerd en de tenaamstelling wordt vergeleken
              met de geregistreerde aanduiding van uw partij
              (IBAN-naam-controle, in deze demo gesimuleerd).
            </p>
          </nldd-rich-text>
          <nldd-form novalidate @submit.prevent="bewaarRekening">
            <nldd-form-field label="IBAN">
              <nldd-text-field
                :value="rekeningForm.iban"
                name="iban"
                placeholder="NL00BANK0123456789"
                @input="rekeningForm.iban = veld($event)"
              ></nldd-text-field>
            </nldd-form-field>
            <nldd-form-field label="Tenaamstelling">
              <nldd-text-field
                :value="rekeningForm.tenaamstelling"
                name="tenaamstelling"
                :placeholder="session.aanvrager?.partij_naam"
                @input="rekeningForm.tenaamstelling = veld($event)"
              ></nldd-text-field>
              <nldd-form-field-help-text>
                De rekening moet op naam van de rechtspersoon staan, niet van
                een bestuurslid of afdeling.
              </nldd-form-field-help-text>
            </nldd-form-field>
            <template v-if="rekeningFout">
              <NBanner variant="critical" :text="rekeningFout" />
            </template>
            <nldd-form-actions>
              <nldd-button
                variant="primary"
                type="submit"
                text="Opslaan"
                :disabled="rekeningBezig || undefined"
              ></nldd-button>
              <nldd-button
                variant="secondary"
                text="Annuleren"
                @click="rekeningOpen = false"
              ></nldd-button>
            </nldd-form-actions>
          </nldd-form>
        </nldd-container>
      </nldd-sheet>
    </template>
  </nldd-page>
</template>
