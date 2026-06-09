<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session } from '../../session.js';
import { euro, onderdelen } from '../../format.js';

const router = useRouter();

const registratie = ref(null);
const geselecteerd = ref(new Set());
const leden = ref(0);

const geenAnoniemeGiften = ref(false);
const geenGiftenNietIngezetenen = ref(false);
const meldplichtNageleefd = ref(false);
const financienOpenbaar = ref(false);

const fout = ref('');
const bezig = ref(false);
const proef = ref(null);
let proefTimer = null;

const GROEPEN = [
  { soort: 'LANDELIJK', titel: 'Landelijk', kolom: 'Kamerzetels (EK + TK)' },
  { orgaan: 'GEMEENTERAAD', titel: 'Gemeenteraden', kolom: 'Raadszetels' },
  { orgaan: 'PROVINCIALE_STATEN', titel: 'Provinciale staten', kolom: 'Statenzetels' },
  { orgaan: 'WATERSCHAP', titel: 'Waterschappen', kolom: 'Zetels algemeen bestuur' },
];

const aanspraken = computed(() => registratie.value?.aanspraken ?? []);

function groepLeden(groep) {
  return aanspraken.value.filter((a) =>
    groep.soort ? a.soort === groep.soort : a.orgaan === groep.orgaan,
  );
}

const zichtbareGroepen = computed(() => GROEPEN.filter((g) => groepLeden(g).length));

const aantalGeselecteerd = computed(() => geselecteerd.value.size);
const landelijkGeselecteerd = computed(() => geselecteerd.value.has('LANDELIJK'));

const verklaringenCompleet = computed(
  () =>
    geenAnoniemeGiften.value &&
    geenGiftenNietIngezetenen.value &&
    meldplichtNageleefd.value &&
    financienOpenbaar.value,
);

function toggle(key) {
  const next = new Set(geselecteerd.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  geselecteerd.value = next;
}

function selecteerGroep(groep, aan) {
  const next = new Set(geselecteerd.value);
  for (const a of groepLeden(groep)) {
    if (a.status !== 'BESCHIKBAAR') continue;
    if (aan) {
      next.add(a.key);
    } else {
      next.delete(a.key);
    }
  }
  geselecteerd.value = next;
}

function statusLabel(a) {
  if (a.status === 'IN_BEHANDELING') return 'Loopt al';
  if (a.status === 'TOEGEKEND') return 'Toegekend';
  return null;
}

function num(event) {
  const v = event.detail?.value ?? Number(event.target?.value);
  return Number.isFinite(v) ? v : 0;
}

async function verstuur() {
  fout.value = '';
  bezig.value = true;
  try {
    const result = await api.nieuweAanvraag({
      componenten: [...geselecteerd.value],
      parameters: {
        aantal_betalende_leden: leden.value,
        ontvangt_anonieme_giften: !geenAnoniemeGiften.value,
        ontvangt_giften_niet_ingezetenen: !geenGiftenNietIngezetenen.value,
        voldoet_aan_meldplicht_giften: meldplichtNageleefd.value,
        financien_openbaar_op_website: financienOpenbaar.value,
      },
    });
    router.push(`/aanvraag/${result.id}`);
  } catch (e) {
    fout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

async function laadRegistratie() {
  if (!session.aanvrager) return;
  try {
    registratie.value = await api.mijnRegistratie();
    // Default: alles wat beschikbaar is staat aangevinkt.
    geselecteerd.value = new Set(
      (registratie.value?.aanspraken ?? [])
        .filter((a) => a.status === 'BESCHIKBAAR')
        .map((a) => a.key),
    );
  } catch {
    registratie.value = null;
  }
}

async function herberekenProef() {
  if (!geselecteerd.value.size) {
    proef.value = null;
    return;
  }
  try {
    proef.value = await api.proefAanspraken({
      componenten: [...geselecteerd.value],
      parameters: {
        aantal_betalende_leden: leden.value,
        ontvangt_anonieme_giften: !geenAnoniemeGiften.value,
        ontvangt_giften_niet_ingezetenen: !geenGiftenNietIngezetenen.value,
        voldoet_aan_meldplicht_giften: meldplichtNageleefd.value,
        financien_openbaar_op_website: financienOpenbaar.value,
      },
    });
  } catch {
    proef.value = null;
  }
}

watch(
  [geselecteerd, leden, geenAnoniemeGiften, geenGiftenNietIngezetenen, meldplichtNageleefd, financienOpenbaar],
  () => {
    clearTimeout(proefTimer);
    proefTimer = setTimeout(herberekenProef, 400);
  },
);

onMounted(laadRegistratie);
watch(() => session.aanvrager, laadRegistratie);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Subsidieportaal politieke partijen"
      :items="[{ text: 'Mijn aanvragen', to: '/' }, { text: 'Nieuwe aanvraag', to: '/nieuw' }]"
    />

    <template v-if="session.loaded && !session.aanvrager">
      <nldd-simple-section width="560px">
        <nldd-inline-dialog
          variant="alert"
          text="U bent niet ingelogd"
          supporting-text="Log eerst in met eHerkenning om een aanvraag in te dienen."
        >
          <nldd-button
            slot="actions"
            variant="primary"
            text="Naar inloggen"
            @click="router.push('/')"
          ></nldd-button>
        </nldd-inline-dialog>
      </nldd-simple-section>
    </template>

    <template v-else>
      <nldd-simple-section width="820px">
        <nldd-title size="2">
          <span slot="overline">{{ session.aanvrager?.partij_naam }}</span>
          <h2>Subsidie aanvragen voor {{ registratie?.subsidiejaar ?? '…' }}</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            Dit zijn uw aanspraken volgens het partijregister, gebaseerd op de
            verkiezingsuitslagen van de Kiesraad. U vraagt alles in één keer aan;
            onderdelen uitvinken kan. De Napp beslist in één beschikking met een
            specificatie per onderdeel.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="24"></nldd-spacer>

        <NBanner
          v-if="registratie && !registratie.partij"
          variant="warning"
          text="Geen registratie gevonden"
          supporting-text="Uw organisatie staat niet in het partijregister van de Napp. U kunt een aanvraag indienen, maar zonder geregistreerde zetels zal de wet haar afwijzen."
        />
        <nldd-spacer v-if="registratie && !registratie.partij" size="16"></nldd-spacer>

        <template v-for="groep in zichtbareGroepen" :key="groep.titel">
          <nldd-title size="4">
            <h3>{{ groep.titel }}</h3>
            <div slot="actions">
              <nldd-button-group orientation="horizontal">
                <nldd-button
                  size="sm"
                  variant="neutral-transparent"
                  text="Alles"
                  @click="selecteerGroep(groep, true)"
                ></nldd-button>
                <nldd-button
                  size="sm"
                  variant="neutral-transparent"
                  text="Niets"
                  @click="selecteerGroep(groep, false)"
                ></nldd-button>
              </nldd-button-group>
            </div>
          </nldd-title>
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-list variant="box">
            <nldd-list-item v-for="a in groepLeden(groep)" :key="a.key" size="sm">
              <nldd-cell width="fit-content">
                <nldd-checkbox
                  :checked="geselecteerd.has(a.key) || undefined"
                  :disabled="a.status !== 'BESCHIKBAAR' || undefined"
                  :accessible-label="`Onderdeel ${a.gebied ?? 'landelijk'}`"
                  @change="toggle(a.key)"
                ></nldd-checkbox>
              </nldd-cell>
              <nldd-spacer-cell size="12"></nldd-spacer-cell>
              <nldd-text-cell
                :text="a.soort === 'LANDELIJK' ? 'Landelijke subsidie' : a.gebied"
                :supporting-text="`${groep.kolom}: ${a.zetels} · bron: Kiesraad`"
              ></nldd-text-cell>
              <template v-if="statusLabel(a)">
                <nldd-cell width="fit-content">
                  <nldd-tag
                    :color="a.status === 'TOEGEKEND' ? 'success' : 'warning'"
                    :text="statusLabel(a)"
                  ></nldd-tag>
                </nldd-cell>
                <nldd-spacer-cell size="8"></nldd-spacer-cell>
              </template>
            </nldd-list-item>
          </nldd-list>
          <nldd-spacer size="24"></nldd-spacer>
        </template>

        <template v-if="landelijkGeselecteerd">
          <nldd-title size="4"><h3>Ledental</h3></nldd-title>
          <nldd-spacer size="8"></nldd-spacer>
          <nldd-form-field label="Betalende leden">
            <nldd-number-field
              :value="leden"
              name="leden"
              min="0"
              step="100"
              @input="leden = num($event)"
              @change="leden = num($event)"
            ></nldd-number-field>
            <nldd-form-field-help-text>
              Eigen opgave, vereist voor de landelijke subsidie: leden die
              jaarlijks ten minste € 12 contributie betalen (minimaal 1.000).
            </nldd-form-field-help-text>
          </nldd-form-field>
          <nldd-spacer size="24"></nldd-spacer>
        </template>

        <nldd-title size="4"><h3>Transparantieverklaringen</h3></nldd-title>
        <nldd-spacer size="4"></nldd-spacer>
        <nldd-rich-text>
          <p>De verklaringen gelden voor de hele rechtspersoon (artikel 5 Wpp).</p>
        </nldd-rich-text>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-container gap="12">
          <nldd-checkbox-field
            label="Onze partij ontvangt geen anonieme giften"
            :checked="geenAnoniemeGiften || undefined"
            @change="geenAnoniemeGiften = $event.detail?.checked ?? false"
          ></nldd-checkbox-field>
          <nldd-checkbox-field
            label="Onze partij ontvangt geen giften van niet-ingezetenen"
            :checked="geenGiftenNietIngezetenen || undefined"
            @change="geenGiftenNietIngezetenen = $event.detail?.checked ?? false"
          ></nldd-checkbox-field>
          <nldd-checkbox-field
            label="Giften van € 10.000 of meer melden wij binnen de termijn"
            :checked="meldplichtNageleefd || undefined"
            @change="meldplichtNageleefd = $event.detail?.checked ?? false"
          ></nldd-checkbox-field>
          <nldd-checkbox-field
            label="Onze financiën staan openbaar op onze website"
            :checked="financienOpenbaar || undefined"
            @change="financienOpenbaar = $event.detail?.checked ?? false"
          ></nldd-checkbox-field>
        </nldd-container>
        <nldd-spacer size="24"></nldd-spacer>

        <template v-if="landelijkGeselecteerd && leden < 1000">
          <NBanner
            variant="warning"
            text="Minder dan duizend leden"
            supporting-text="Voor de landelijke subsidie zijn minimaal 1.000 betalende leden vereist (artikel 6 Wpp). Met dit ledental wijst de wet het landelijke onderdeel af."
          />
          <nldd-spacer size="16"></nldd-spacer>
        </template>
        <template v-if="!verklaringenCompleet">
          <NBanner
            variant="neutral"
            text="Let op"
            supporting-text="Een aanvraag zonder volledige verklaringen wordt door de wet afgewezen. U kunt wel indienen; het besluit volgt uit de wet."
          />
          <nldd-spacer size="16"></nldd-spacer>
        </template>

        <template v-if="proef">
          <nldd-box>
            <nldd-container padding="16">
              <nldd-text-cell
                overline="Indicatieve uitkomst volgens de wet"
                :text="proef.subsidie_toegekend ? euro(proef.subsidiebedrag) : 'Afwijzing'"
                :supporting-text="`${proef.onderdelen_toegekend} van ${onderdelen(proef.onderdelen_totaal)} toegekend · dit is geen besluit`"
                size="md"
              ></nldd-text-cell>
            </nldd-container>
          </nldd-box>
          <nldd-spacer size="16"></nldd-spacer>
        </template>

        <nldd-button-group orientation="horizontal">
          <nldd-button
            variant="primary"
            :text="`Aanvraag indienen (${onderdelen(aantalGeselecteerd)})`"
            start-icon="paper-plane"
            :disabled="bezig || aantalGeselecteerd === 0 || undefined"
            @click="verstuur"
          ></nldd-button>
          <nldd-button
            variant="neutral-transparent"
            text="Annuleren"
            @click="router.push('/')"
          ></nldd-button>
        </nldd-button-group>

        <template v-if="fout">
          <nldd-spacer size="16"></nldd-spacer>
          <NBanner variant="critical" text="Indienen mislukt" :supporting-text="fout" />
        </template>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
