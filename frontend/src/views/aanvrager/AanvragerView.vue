<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { api } from '../../api.js';
import { session, refreshSession } from '../../session.js';
import { euro, datum, statusLabel, statusColor } from '../../format.js';

const router = useRouter();

const kvk = ref('');
const partij = ref('');
const loginFout = ref('');
const bezig = ref(false);

const aanvragen = ref([]);
const laden = ref(false);

const navItems = computed(() =>
  session.aanvrager
    ? [
        { text: 'Mijn aanvragen', to: '/' },
        { text: 'Nieuwe aanvraag', to: '/nieuw' },
      ]
    : [],
);
const utilityItems = computed(() =>
  session.aanvrager
    ? [{ text: `Uitloggen (${session.aanvrager.partij_naam})`, icon: 'logout', key: 'logout' }]
    : [],
);

async function login() {
  loginFout.value = '';
  if (!/^\d{8}$/.test(kvk.value.trim())) {
    loginFout.value = 'Vul een geldig KVK-nummer in (8 cijfers).';
    return;
  }
  if (!partij.value.trim()) {
    loginFout.value = 'Vul de naam van uw partij in.';
    return;
  }
  bezig.value = true;
  try {
    await api.eherkenningLogin(kvk.value.trim(), partij.value.trim());
    await refreshSession();
  } catch (e) {
    loginFout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

async function logout() {
  await api.eherkenningLogout();
  await refreshSession();
  aanvragen.value = [];
}

async function laadAanvragen() {
  if (!session.aanvrager) return;
  laden.value = true;
  try {
    aanvragen.value = await api.aanvragen();
  } finally {
    laden.value = false;
  }
}

onMounted(laadAanvragen);
watch(() => session.aanvrager, laadAanvragen);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Subsidieportaal politieke partijen"
      :items="navItems"
      :utility-items="utilityItems"
      @utility="logout"
    />

    <!-- Niet ingelogd: eHerkenning (gemockt) -->
    <template v-if="session.loaded && !session.aanvrager">
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
          supporting-text="eHerkenning is in deze demonstratie gesimuleerd. Vul een fictief KVK-nummer en partijnaam in."
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
              <nldd-form-field label="Naam politieke partij">
                <nldd-text-field
                  name="partij"
                  :value="partij"
                  placeholder="Bijvoorbeeld: Partij voor de Demo"
                  @input="partij = $event.detail?.value ?? $event.target?.value ?? ''"
                ></nldd-text-field>
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
      </nldd-simple-section>
    </template>

    <!-- Ingelogd: eigen aanvragen -->
    <template v-else-if="session.aanvrager">
      <nldd-simple-section>
        <nldd-title size="2">
          <span slot="overline">{{ session.aanvrager.partij_naam }} · KVK {{ session.aanvrager.kvk_nummer }}</span>
          <h2>Uw subsidieaanvragen</h2>
          <div slot="actions">
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
              :text="item.aanvraag.niveau === 'LANDELIJK' ? 'Landelijke subsidie' : 'Decentrale subsidie'"
              :supporting-text="`Ingediend op ${datum(item.aanvraag.aanvraag_datum)}`"
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
