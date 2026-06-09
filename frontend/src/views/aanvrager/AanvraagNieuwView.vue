<script setup>
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session } from '../../session.js';

const router = useRouter();

const niveau = ref('LANDELIJK');
const kamerzetels = ref(0);
const leden = ref(0);
const raadszetels = ref(0);
const gemeente = ref('');
const inwoners = ref(0);

// Verklaringen, positief geformuleerd; de wet kent verboden, dus de eerste
// twee worden geinverteerd verstuurd.
const geenAnoniemeGiften = ref(false);
const geenGiftenNietIngezetenen = ref(false);
const meldplichtNageleefd = ref(false);
const financienOpenbaar = ref(false);

const fout = ref('');
const bezig = ref(false);

const verklaringenCompleet = computed(
  () =>
    geenAnoniemeGiften.value &&
    geenGiftenNietIngezetenen.value &&
    meldplichtNageleefd.value &&
    financienOpenbaar.value,
);

function num(event) {
  const v = event.detail?.value ?? Number(event.target?.value);
  return Number.isFinite(v) ? v : 0;
}

async function verstuur() {
  fout.value = '';
  bezig.value = true;
  try {
    const parameters = {
      aantal_kamerzetels: kamerzetels.value,
      aantal_betalende_leden: leden.value,
      aantal_raadszetels: raadszetels.value,
      inwoneraantal_gemeente: inwoners.value,
      ontvangt_anonieme_giften: !geenAnoniemeGiften.value,
      ontvangt_giften_niet_ingezetenen: !geenGiftenNietIngezetenen.value,
      voldoet_aan_meldplicht_giften: meldplichtNageleefd.value,
      financien_openbaar_op_website: financienOpenbaar.value,
    };
    const result = await api.nieuweAanvraag({
      niveau: niveau.value,
      gemeente: niveau.value === 'DECENTRAAL' ? gemeente.value : null,
      parameters,
    });
    router.push(`/aanvraag/${result.id}`);
  } catch (e) {
    fout.value = e.message;
  } finally {
    bezig.value = false;
  }
}
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
      <nldd-simple-section width="720px">
        <nldd-title size="2">
          <span slot="overline">{{ session.aanvrager?.partij_naam }}</span>
          <h2>Subsidie aanvragen</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            De Napp toetst uw aanvraag aan de Wet op de politieke partijen. U ontvangt
            een besluit met motivering; daarna geldt een bezwaartermijn van zes weken.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-form novalidate @submit.prevent="verstuur">
          <nldd-form-section
            text="Soort subsidie"
            supporting-text="Landelijke en decentrale partijen kennen elk een eigen berekening."
          >
            <nldd-form-field label="Niveau">
              <nldd-segmented-control
                :value="niveau"
                name="niveau"
                width="full"
                @change="niveau = $event.detail?.value ?? niveau"
              >
                <nldd-segmented-control-item value="LANDELIJK" text="Landelijk"></nldd-segmented-control-item>
                <nldd-segmented-control-item value="DECENTRAAL" text="Decentraal"></nldd-segmented-control-item>
              </nldd-segmented-control>
            </nldd-form-field>
          </nldd-form-section>

          <nldd-form-section
            v-if="niveau === 'LANDELIJK'"
            text="Vertegenwoordiging en leden"
            supporting-text="Vereist: minimaal 1 kamerzetel en 1.000 betalende leden."
          >
            <nldd-form-field label="Zetels in Eerste en Tweede Kamer">
              <nldd-number-field
                :value="kamerzetels"
                name="kamerzetels"
                min="0"
                max="225"
                @input="kamerzetels = num($event)"
                @change="kamerzetels = num($event)"
              ></nldd-number-field>
            </nldd-form-field>
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
                Leden die jaarlijks ten minste € 12 contributie betalen.
              </nldd-form-field-help-text>
            </nldd-form-field>
          </nldd-form-section>

          <nldd-form-section
            v-else
            text="Vertegenwoordiging"
            supporting-text="Vereist: minimaal 1 zetel bij de laatste gemeenteraads- of statenverkiezing."
          >
            <nldd-form-field label="Gemeente of provincie">
              <nldd-text-field
                :value="gemeente"
                name="gemeente"
                placeholder="Bijvoorbeeld: Utrecht"
                @input="gemeente = $event.detail?.value ?? $event.target?.value ?? ''"
              ></nldd-text-field>
            </nldd-form-field>
            <nldd-form-field label="Behaalde raads- of statenzetels">
              <nldd-number-field
                :value="raadszetels"
                name="raadszetels"
                min="0"
                max="55"
                @input="raadszetels = num($event)"
                @change="raadszetels = num($event)"
              ></nldd-number-field>
            </nldd-form-field>
            <nldd-form-field label="Inwoneraantal gemeente">
              <nldd-number-field
                :value="inwoners"
                name="inwoners"
                min="0"
                step="1000"
                @input="inwoners = num($event)"
                @change="inwoners = num($event)"
              ></nldd-number-field>
              <nldd-form-field-help-text>
                Het bedrag per zetel hangt af van het inwoneraantal.
              </nldd-form-field-help-text>
            </nldd-form-field>
          </nldd-form-section>

          <nldd-form-section
            text="Transparantieverklaringen"
            supporting-text="Alle verklaringen zijn verplicht (artikel 5 Wpp)."
          >
            <nldd-form-field>
              <nldd-checkbox-field
                label="Onze partij ontvangt geen anonieme giften"
                :checked="geenAnoniemeGiften || undefined"
                @change="geenAnoniemeGiften = $event.detail?.checked ?? false"
              ></nldd-checkbox-field>
            </nldd-form-field>
            <nldd-form-field>
              <nldd-checkbox-field
                label="Onze partij ontvangt geen giften van niet-ingezetenen"
                :checked="geenGiftenNietIngezetenen || undefined"
                @change="geenGiftenNietIngezetenen = $event.detail?.checked ?? false"
              ></nldd-checkbox-field>
            </nldd-form-field>
            <nldd-form-field>
              <nldd-checkbox-field
                label="Giften van € 10.000 of meer melden wij binnen de termijn"
                :checked="meldplichtNageleefd || undefined"
                @change="meldplichtNageleefd = $event.detail?.checked ?? false"
              ></nldd-checkbox-field>
            </nldd-form-field>
            <nldd-form-field>
              <nldd-checkbox-field
                label="Onze financiën staan openbaar op onze website"
                :checked="financienOpenbaar || undefined"
                @change="financienOpenbaar = $event.detail?.checked ?? false"
              ></nldd-checkbox-field>
            </nldd-form-field>
          </nldd-form-section>

          <template v-if="!verklaringenCompleet">
            <NBanner
              variant="neutral"
              text="Let op"
              supporting-text="Een aanvraag zonder volledige verklaringen wordt door de wet afgewezen. U kunt wel indienen; het besluit volgt uit de wet."
            />
            <nldd-spacer size="16"></nldd-spacer>
          </template>

          <nldd-form-actions>
            <nldd-button-group orientation="horizontal">
              <nldd-button
                variant="primary"
                type="submit"
                text="Aanvraag indienen"
                start-icon="paper-plane"
                :disabled="bezig || undefined"
              ></nldd-button>
              <nldd-button
                variant="neutral-transparent"
                text="Annuleren"
                @click="router.push('/')"
              ></nldd-button>
            </nldd-button-group>
          </nldd-form-actions>
        </nldd-form>

        <template v-if="fout">
          <nldd-spacer size="16"></nldd-spacer>
          <NBanner variant="critical" text="Indienen mislukt" :supporting-text="fout" />
        </template>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
