<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session } from '../../session.js';

const router = useRouter();

const niveau = ref('LANDELIJK');
const leden = ref(0);
const gemeente = ref('');

// Registergegevens (Kiesraad/CBS) van de ingelogde partij.
const registratie = ref(null);

// Verklaringen, positief geformuleerd; de wet kent verboden, dus de eerste
// twee worden geinverteerd verstuurd.
const geenAnoniemeGiften = ref(false);
const geenGiftenNietIngezetenen = ref(false);
const meldplichtNageleefd = ref(false);
const financienOpenbaar = ref(false);

const fout = ref('');
const bezig = ref(false);

const geregistreerd = computed(() => Boolean(registratie.value?.partij));
const kamerzetels = computed(() => registratie.value?.partij?.kamerzetels ?? 0);

const uitslag = computed(() =>
  registratie.value?.decentrale_uitslagen?.find((u) => u.gebied_code === gemeente.value) ?? null,
);
const inwoneraantal = computed(
  () =>
    registratie.value?.gebieden?.find((g) => g.code === gemeente.value)?.inwoneraantal ?? null,
);

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
      aantal_betalende_leden: leden.value,
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

async function laadRegistratie() {
  if (!session.aanvrager) return;
  try {
    registratie.value = await api.mijnRegistratie();
  } catch {
    registratie.value = null;
  }
}

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
      <nldd-simple-section width="720px">
        <nldd-title size="2">
          <span slot="overline">{{ session.aanvrager?.partij_naam }}</span>
          <h2>Subsidie aanvragen</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            De Napp toetst uw aanvraag aan de Wet op de politieke partijen.
            Zetelaantallen komen uit de uitslagen van de Kiesraad en
            inwoneraantallen uit de cijfers van het CBS; die hoeft u niet zelf
            op te geven. U ontvangt een besluit met motivering; daarna geldt een
            bezwaartermijn van zes weken.
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
            <template v-if="registratie && !geregistreerd">
              <NBanner
                variant="warning"
                text="Geen registratie gevonden"
                supporting-text="Uw organisatie staat niet in het partijregister van de Napp. U kunt de aanvraag indienen, maar zonder geregistreerde kamerzetels zal de wet haar afwijzen."
              />
              <nldd-spacer size="16"></nldd-spacer>
            </template>
            <template v-else-if="registratie && kamerzetels === 0">
              <NBanner
                variant="warning"
                text="Geen kamerzetels"
                supporting-text="Volgens de uitslagen van de Kiesraad heeft uw partij geen zetels in de Eerste of Tweede Kamer. U kunt de aanvraag indienen, maar de wet zal haar afwijzen."
              />
              <nldd-spacer size="16"></nldd-spacer>
            </template>
            <nldd-form-field label="Zetels in Eerste en Tweede Kamer">
              <nldd-text-field
                :value="String(kamerzetels)"
                readonly
              ></nldd-text-field>
              <nldd-form-field-help-text>
                Bron: uitslagen Kiesraad, via het partijregister van de Napp.
              </nldd-form-field-help-text>
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
                Eigen opgave: leden die jaarlijks ten minste € 12 contributie
                betalen.
              </nldd-form-field-help-text>
            </nldd-form-field>
          </nldd-form-section>

          <nldd-form-section
            v-else
            text="Vertegenwoordiging"
            supporting-text="Vereist: minimaal 1 zetel bij de laatste gemeenteraads- of statenverkiezing."
          >
            <nldd-form-field label="Gemeente">
              <nldd-dropdown>
                <select :value="gemeente" @change="gemeente = $event.target.value">
                  <option value="" disabled>Kies een gemeente</option>
                  <option
                    v-for="g in registratie?.gebieden ?? []"
                    :key="g.code"
                    :value="g.code"
                  >
                    {{ g.naam }}
                  </option>
                </select>
              </nldd-dropdown>
            </nldd-form-field>
            <template v-if="gemeente">
              <template v-if="!uitslag">
                <NBanner
                  variant="warning"
                  text="Geen zetels in deze gemeente"
                  supporting-text="Volgens de uitslagen van de Kiesraad heeft uw partij in deze gemeente geen raadszetels. U kunt de aanvraag indienen, maar de wet zal haar afwijzen."
                />
                <nldd-spacer size="16"></nldd-spacer>
              </template>
              <nldd-form-field label="Behaalde raadszetels">
                <nldd-text-field :value="String(uitslag?.zetels ?? 0)" readonly></nldd-text-field>
                <nldd-form-field-help-text>
                  Bron: uitslagen Kiesraad.
                </nldd-form-field-help-text>
              </nldd-form-field>
              <nldd-form-field label="Inwoneraantal gemeente">
                <nldd-text-field :value="String(inwoneraantal ?? 0)" readonly></nldd-text-field>
                <nldd-form-field-help-text>
                  Bron: CBS. Bepaalt het bedrag per zetel.
                </nldd-form-field-help-text>
              </nldd-form-field>
            </template>
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
                :disabled="bezig || (niveau === 'DECENTRAAL' && !gemeente) || undefined"
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
