<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import { getEngine } from '../../engine.js';
import { runFeature } from '../../gherkin/runner.js';

const router = useRouter();

// Bundel alle scenario's mee als raw tekst.
const featureFiles = import.meta.glob('../../../../scenarios/*.feature', {
  query: '?raw',
  import: 'default',
  eager: true,
});

const resultaten = ref([]);
const bezig = ref(false);
const fout = ref('');
const klaar = ref(false);
const open = ref(new Set());

const totaal = computed(() =>
  resultaten.value.reduce((n, f) => n + f.scenarios.length, 0),
);
const geslaagd = computed(() =>
  resultaten.value.reduce((n, f) => n + f.scenarios.filter((s) => s.passed).length, 0),
);

function sleutel(feature, scenario) {
  return `${feature}::${scenario}`;
}

function toggle(feature, scenario) {
  const k = sleutel(feature, scenario);
  const next = new Set(open.value);
  if (next.has(k)) {
    next.delete(k);
  } else {
    next.add(k);
  }
  open.value = next;
}

function stapIcoon(status) {
  if (status === 'geslaagd') return 'check-mark-circle';
  if (status === 'mislukt') return 'dismiss-circle';
  return 'circle-dashed';
}

function stapKleur(status) {
  if (status === 'geslaagd') return 'success';
  if (status === 'mislukt') return 'critical';
  return 'secondary';
}

function tabelTekst(step) {
  if (!step.dataTable) return '';
  return step.dataTable.map((row) => `| ${row.join(' | ')} |`).join('\n');
}

async function run() {
  bezig.value = true;
  fout.value = '';
  resultaten.value = [];
  klaar.value = false;
  try {
    const engine = await getEngine();
    const namen = Object.keys(featureFiles).sort();
    for (const naam of namen) {
      const uitkomst = await runFeature(featureFiles[naam], engine);
      resultaten.value.push(uitkomst);
    }
    klaar.value = true;
  } catch (e) {
    fout.value = String(e?.message ?? e);
  } finally {
    bezig.value = false;
  }
}

onMounted(run);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Beoordelingsomgeving"
      :items="[{ text: 'Werkvoorraad', to: '/' }, { text: 'Scenario’s', to: '/scenarios' }]"
    />

    <nldd-simple-section width="860px">
      <nldd-button
        variant="neutral-transparent"
        size="sm"
        text="Terug naar de werkvoorraad"
        start-icon="chevron-left"
        @click="router.push('/')"
      ></nldd-button>
      <nldd-spacer size="16"></nldd-spacer>

      <nldd-title size="2">
        <span slot="overline">Doet de wet wat hij moet doen?</span>
        <h2>Scenario's</h2>
        <div slot="actions">
          <nldd-button
            variant="secondary"
            text="Opnieuw uitvoeren"
            start-icon="arrow-2-counter-clockwise"
            :disabled="bezig || undefined"
            @click="run"
          ></nldd-button>
        </div>
      </nldd-title>
      <nldd-spacer size="12"></nldd-spacer>
      <nldd-rich-text>
        <p>
          Deze scenario's beschrijven hoe de Wet op de politieke partijen hoort te
          werken: welke partijen subsidie krijgen, welke aanvragen worden afgewezen
          en welke AWB-regels aanhaken. Ze worden hier live uitgevoerd door dezelfde
          wet-engine die de besluiten neemt — in uw browser. Klik op een scenario
          voor de volledige stappen.
        </p>
      </nldd-rich-text>
      <nldd-spacer size="24"></nldd-spacer>

      <NBanner v-if="fout" variant="critical" text="Uitvoeren mislukt" :supporting-text="fout" />

      <NBanner
        v-else-if="klaar"
        :variant="geslaagd === totaal ? 'success' : 'critical'"
        :text="`${geslaagd} van ${totaal} scenario's geslaagd`"
        :supporting-text="geslaagd === totaal
          ? 'De wet gedraagt zich in alle beschreven situaties zoals bedoeld.'
          : 'Niet alle scenario\'s slagen — de wet of de scenario\'s vragen aandacht.'"
      />

      <nldd-activity-indicator v-else-if="bezig" show-text text="Scenario's uitvoeren"></nldd-activity-indicator>

      <template v-for="feature in resultaten" :key="feature.feature">
        <nldd-spacer size="32"></nldd-spacer>
        <nldd-title size="4"><h3>{{ feature.feature }}</h3></nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <nldd-list variant="box">
          <template v-for="scenario in feature.scenarios" :key="scenario.name">
            <nldd-list-item
              size="md"
              type="button"
              @click="toggle(feature.feature, scenario.name)"
            >
              <nldd-icon-cell
                :icon="scenario.passed ? 'check-mark-circle' : 'dismiss-circle'"
                :color="scenario.passed ? 'success' : 'critical'"
                size="20"
              ></nldd-icon-cell>
              <nldd-text-cell :text="scenario.name">
                <span v-if="!scenario.passed" slot="supporting-text">
                  {{ scenario.steps.find((s) => s.status === 'mislukt')?.error }}
                </span>
              </nldd-text-cell>
              <nldd-cell width="fit-content">
                <nldd-tag
                  :color="scenario.passed ? 'success' : 'critical'"
                  :text="scenario.passed ? 'Geslaagd' : 'Mislukt'"
                ></nldd-tag>
              </nldd-cell>
              <nldd-icon-cell
                :icon="open.has(sleutel(feature.feature, scenario.name)) ? 'chevron-up' : 'chevron-down'"
                size="16"
                color="secondary"
              ></nldd-icon-cell>
            </nldd-list-item>

            <template v-if="open.has(sleutel(feature.feature, scenario.name))">
              <template v-for="(step, si) in scenario.steps" :key="si">
                <nldd-list-item size="sm">
                  <nldd-spacer-cell size="24"></nldd-spacer-cell>
                  <nldd-icon-cell
                    :icon="stapIcoon(step.status)"
                    :color="stapKleur(step.status)"
                    size="16"
                  ></nldd-icon-cell>
                  <nldd-text-cell size="sm" :text="`**${step.keyword}** ${step.text}`">
                    <span v-if="step.error" slot="supporting-text">{{ step.error }}</span>
                  </nldd-text-cell>
                </nldd-list-item>
                <nldd-list-item v-if="step.dataTable" size="sm">
                  <nldd-spacer-cell size="48"></nldd-spacer-cell>
                  <nldd-cell width="full" vertical-alignment="top">
                    <nldd-code-viewer variant="simple" no-copy>{{ tabelTekst(step) }}</nldd-code-viewer>
                  </nldd-cell>
                </nldd-list-item>
              </template>
            </template>
          </template>
        </nldd-list>
      </template>
    </nldd-simple-section>
  </nldd-page>
</template>
