<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session, refreshSession } from '../../session.js';
import { euro, datum, statusLabel, statusColor } from '../../format.js';

const router = useRouter();

const naam = ref('');
const loginFout = ref('');
const tab = ref('werkvoorraad');

const navItems = computed(() =>
  session.beoordelaar
    ? [
        { text: 'Werkvoorraad', to: '/' },
        { text: "Scenario's", to: '/scenarios' },
      ]
    : [],
);
const utilityItems = computed(() =>
  session.beoordelaar
    ? [{ text: `${session.beoordelaar.naam}`, icon: 'person', key: 'noop' }]
    : [],
);
function onUtility() {}

const aanvragen = ref([]);
const betaalopdrachten = ref([]);

const openstaand = computed(() =>
  aanvragen.value.filter((i) => i.aanvraag.status === 'BEHANDELING'),
);
const afgerond = computed(() =>
  aanvragen.value.filter((i) => i.aanvraag.status !== 'BEHANDELING'),
);

async function mockLogin() {
  loginFout.value = '';
  if (!naam.value.trim()) {
    loginFout.value = 'Vul uw naam in.';
    return;
  }
  try {
    await api.ssoMockLogin(naam.value.trim());
    await refreshSession();
  } catch (e) {
    loginFout.value = e.message;
  }
}

async function laad() {
  if (!session.beoordelaar) return;
  aanvragen.value = await api.aanvragen();
  betaalopdrachten.value = await api.betaalopdrachten();
}

onMounted(laad);
watch(() => session.beoordelaar, laad);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Beoordelingsomgeving"
      :items="navItems"
      :utility-items="utilityItems"
      @utility="onUtility"
    />

    <!-- Niet ingelogd: SSO Rijk -->
    <template v-if="session.loaded && !session.beoordelaar">
      <nldd-simple-section width="560px">
        <nldd-title size="2">
          <span slot="overline">Voor medewerkers van de Napp</span>
          <h2>Inloggen met SSO Rijk</h2>
        </nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-rich-text>
          <p>
            Beoordelaars van de Nederlandse autoriteit politieke partijen loggen in
            met hun Rijksaccount.
          </p>
        </nldd-rich-text>
        <nldd-spacer size="24"></nldd-spacer>
        <nldd-card accessible-label="SSO Rijk login">
          <nldd-container padding="24" gap="16">
            <nldd-button
              variant="primary"
              text="Inloggen met SSO Rijk"
              start-icon="shield-check-mark"
              href="/auth/login"
              width="full"
            ></nldd-button>
            <nldd-divider></nldd-divider>
            <NBanner
              variant="warning"
              text="Demo zonder SSO-koppeling"
              supporting-text="Geen Rijksaccount in deze omgeving? Gebruik de demo-login hieronder."
            />
            <nldd-form novalidate @submit.prevent="mockLogin">
              <nldd-form-field label="Uw naam (demo)">
                <nldd-text-field
                  :value="naam"
                  name="naam"
                  placeholder="Bijvoorbeeld: A. de Beoordelaar"
                  @input="naam = $event.detail?.value ?? $event.target?.value ?? ''"
                ></nldd-text-field>
              </nldd-form-field>
              <nldd-form-actions>
                <nldd-button variant="secondary" type="submit" text="Demo-login"></nldd-button>
              </nldd-form-actions>
            </nldd-form>
            <template v-if="loginFout">
              <NBanner variant="critical" :text="loginFout" />
            </template>
          </nldd-container>
        </nldd-card>
      </nldd-simple-section>
    </template>

    <!-- Ingelogd: werkvoorraad -->
    <template v-else-if="session.beoordelaar">
      <nldd-simple-section>
        <nldd-title size="2">
          <span slot="overline">Beoordelingsomgeving</span>
          <h2>Subsidieaanvragen</h2>
          <div slot="actions">
            <nldd-button
              variant="secondary"
              text="Scenario's: werkt de wet?"
              start-icon="check-list"
              @click="router.push('/scenarios')"
            ></nldd-button>
          </div>
        </nldd-title>
        <nldd-spacer size="16"></nldd-spacer>

        <nldd-segmented-control
          :value="tab"
          width="fit-content"
          @change="tab = $event.detail?.value ?? tab"
        >
          <nldd-segmented-control-item value="werkvoorraad" :text="`Werkvoorraad (${openstaand.length})`"></nldd-segmented-control-item>
          <nldd-segmented-control-item value="afgerond" :text="`Besloten (${afgerond.length})`"></nldd-segmented-control-item>
          <nldd-segmented-control-item value="betalingen" :text="`Betaalopdrachten (${betaalopdrachten.length})`"></nldd-segmented-control-item>
        </nldd-segmented-control>
        <nldd-spacer size="24"></nldd-spacer>

        <template v-if="tab === 'werkvoorraad' || tab === 'afgerond'">
          <nldd-list
            v-if="(tab === 'werkvoorraad' ? openstaand : afgerond).length"
            variant="box"
          >
            <nldd-list-item
              v-for="item in tab === 'werkvoorraad' ? openstaand : afgerond"
              :key="item.aanvraag.id"
              size="md"
              type="button"
              @click="router.push(`/aanvraag/${item.aanvraag.id}`)"
            >
              <nldd-title-cell
                :text="item.aanvraag.partij_naam"
                :overline="item.aanvraag.niveau === 'LANDELIJK' ? 'Landelijk' : `Decentraal · ${item.aanvraag.gemeente ?? ''}`"
                :supporting-text="`KVK ${item.aanvraag.kvk_nummer} · ingediend ${datum(item.aanvraag.aanvraag_datum)}`"
              ></nldd-title-cell>
              <nldd-text-cell
                v-if="item.besluit"
                width="fit-content"
                :text="euro(item.besluit.subsidiebedrag)"
                horizontal-alignment="right"
              ></nldd-text-cell>
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
            v-else
            icon="inbox"
            :text="tab === 'werkvoorraad' ? 'Geen openstaande aanvragen' : 'Nog geen besluiten'"
            supporting-text="Nieuwe aanvragen verschijnen hier automatisch."
          ></nldd-inline-dialog>
        </template>

        <template v-else>
          <nldd-table
            v-if="betaalopdrachten.length"
            columns="minmax(220px,1fr) 160px 140px 180px"
            accessible-label="Betaalopdrachten"
          >
            <nldd-table-row slot="header">
              <nldd-text-cell text="Partij"></nldd-text-cell>
              <nldd-text-cell text="Bedrag" horizontal-alignment="right"></nldd-text-cell>
              <nldd-text-cell text="Status"></nldd-text-cell>
              <nldd-text-cell text="Aangemaakt"></nldd-text-cell>
            </nldd-table-row>
            <nldd-table-row v-for="opdracht in betaalopdrachten" :key="opdracht.id">
              <nldd-text-cell :text="opdracht.partij_naam"></nldd-text-cell>
              <nldd-text-cell :text="euro(opdracht.bedrag)" horizontal-alignment="right"></nldd-text-cell>
              <nldd-cell>
                <nldd-tag color="accent" :text="opdracht.status === 'AANGEMAAKT' ? 'Aangemaakt' : opdracht.status"></nldd-tag>
              </nldd-cell>
              <nldd-text-cell :text="opdracht.created_at" color="secondary"></nldd-text-cell>
            </nldd-table-row>
          </nldd-table>
          <nldd-inline-dialog
            v-else
            icon="euro-sign"
            text="Nog geen betaalopdrachten"
            supporting-text="Bij een toekennend besluit ontstaat automatisch een betaalopdracht (artikel 16 Wpp)."
          ></nldd-inline-dialog>
        </template>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
