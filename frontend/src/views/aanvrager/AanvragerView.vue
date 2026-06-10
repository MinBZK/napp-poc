<script setup>
import { computed, onMounted, ref, watch } from 'vue';
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

onMounted(async () => {
  laadAanvragen();
  try {
    demoVoorbeelden.value = await api.registerDemo();
  } catch {
    demoVoorbeelden.value = [];
  }
});
watch(() => session.aanvrager, laadAanvragen);
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
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
