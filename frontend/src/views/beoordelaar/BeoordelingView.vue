<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import LifecycleTimeline from '../../components/LifecycleTimeline.vue';
import { api } from '../../api.js';
import { euro, datum } from '../../format.js';

const route = useRoute();
const router = useRouter();

const item = ref(null);
const uitkomst = ref(null);
const fout = ref('');
const bezig = ref(false);

const params = computed(() => item.value?.aanvraag.parameters ?? {});

const gegevens = computed(() => {
  const p = params.value;
  const a = item.value?.aanvraag;
  if (!a) return [];
  const rows = [
    { label: 'Partij', waarde: a.partij_naam },
    { label: 'KVK-nummer', waarde: a.kvk_nummer },
    { label: 'Niveau', waarde: a.niveau === 'LANDELIJK' ? 'Landelijk' : 'Decentraal' },
  ];
  if (a.niveau === 'LANDELIJK') {
    rows.push(
      { label: 'Kamerzetels (EK + TK)', waarde: String(p.aantal_kamerzetels ?? 0) },
      { label: 'Betalende leden', waarde: String(p.aantal_betalende_leden ?? 0) },
    );
  } else {
    rows.push(
      { label: 'Gemeente of provincie', waarde: a.gemeente || 'Onbekend' },
      { label: 'Raads- of statenzetels', waarde: String(p.aantal_raadszetels ?? 0) },
      { label: 'Inwoneraantal gemeente', waarde: String(p.inwoneraantal_gemeente ?? 0) },
    );
  }
  return rows;
});

const verklaringen = computed(() => {
  const p = params.value;
  return [
    { label: 'Geen anonieme giften', ok: p.ontvangt_anonieme_giften === false },
    { label: 'Geen giften van niet-ingezetenen', ok: p.ontvangt_giften_niet_ingezetenen === false },
    { label: 'Meldplicht giften ≥ € 10.000 nageleefd', ok: p.voldoet_aan_meldplicht_giften === true },
    { label: 'Financiën openbaar op website', ok: p.financien_openbaar_op_website === true },
  ];
});

async function laad() {
  try {
    item.value = await api.aanvraag(route.params.id);
    if (item.value.aanvraag.status === 'BEHANDELING') {
      uitkomst.value = await api.proefberekening(route.params.id);
    }
  } catch (e) {
    fout.value = e.message;
  }
}

async function stelVast() {
  bezig.value = true;
  fout.value = '';
  try {
    await api.stelBesluitVast(route.params.id);
    uitkomst.value = null;
    await laad();
  } catch (e) {
    fout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

async function maakBekend() {
  bezig.value = true;
  fout.value = '';
  try {
    await api.bekendmaking(route.params.id);
    await laad();
  } catch (e) {
    fout.value = e.message;
  } finally {
    bezig.value = false;
  }
}

onMounted(laad);
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Beoordelingsomgeving"
      :items="[{ text: 'Werkvoorraad', to: '/' }, { text: 'Scenario\u2019s', to: '/scenarios' }]"
    />

    <template v-if="item">
      <nldd-simple-section>
        <nldd-button
          variant="neutral-transparent"
          size="sm"
          text="Terug naar de werkvoorraad"
          start-icon="chevron-left"
          @click="router.push('/')"
        ></nldd-button>
        <nldd-spacer size="16"></nldd-spacer>

        <nldd-title size="2">
          <span slot="overline">Beoordeling subsidieaanvraag</span>
          <h2>{{ item.aanvraag.partij_naam }}</h2>
        </nldd-title>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-one-half-one-half-section padding-block="0">
          <div slot="left">
            <nldd-title size="4"><h3>Aanvraaggegevens</h3></nldd-title>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-list variant="box">
              <nldd-list-item v-for="rij in gegevens" :key="rij.label" size="sm">
                <nldd-text-cell :text="rij.label" color="secondary"></nldd-text-cell>
                <nldd-text-cell :text="rij.waarde" horizontal-alignment="right"></nldd-text-cell>
              </nldd-list-item>
            </nldd-list>
            <nldd-spacer size="24"></nldd-spacer>

            <nldd-title size="4"><h3>Transparantieverklaringen (art. 5)</h3></nldd-title>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-list variant="box">
              <nldd-list-item v-for="v in verklaringen" :key="v.label" size="sm">
                <nldd-icon-cell
                  :icon="v.ok ? 'check-mark-circle' : 'dismiss-circle'"
                  :color="v.ok ? 'success' : 'critical'"
                  size="20"
                ></nldd-icon-cell>
                <nldd-spacer-cell size="8"></nldd-spacer-cell>
                <nldd-text-cell :text="v.label"></nldd-text-cell>
              </nldd-list-item>
            </nldd-list>
          </div>

          <div slot="right">
            <!-- Nog te besluiten: proefberekening + vaststellen -->
            <template v-if="item.aanvraag.status === 'BEHANDELING' && uitkomst">
              <nldd-title size="4"><h3>Uitkomst volgens de wet</h3></nldd-title>
              <nldd-spacer size="12"></nldd-spacer>
              <NBanner
                :variant="uitkomst.subsidie_toegekend ? 'success' : 'critical'"
                :text="uitkomst.subsidie_toegekend
                  ? `Toekenning: ${euro(uitkomst.subsidiebedrag)}`
                  : 'Afwijzing'"
                :supporting-text="uitkomst.motivering"
              />
              <nldd-spacer size="16"></nldd-spacer>
              <nldd-box>
                <nldd-container padding="16" gap="4">
                  <nldd-list variant="simple">
                    <nldd-list-item size="sm">
                      <nldd-icon-cell
                        :icon="uitkomst.voldoet_aan_transparantie ? 'check-mark-circle' : 'dismiss-circle'"
                        :color="uitkomst.voldoet_aan_transparantie ? 'success' : 'critical'"
                        size="20"
                      ></nldd-icon-cell>
                      <nldd-spacer-cell size="8"></nldd-spacer-cell>
                      <nldd-text-cell text="Transparantie-eisen (art. 5)"></nldd-text-cell>
                    </nldd-list-item>
                    <nldd-list-item size="sm">
                      <nldd-icon-cell
                        :icon="(item.aanvraag.niveau === 'LANDELIJK' ? uitkomst.heeft_recht_landelijk : uitkomst.heeft_recht_decentraal) ? 'check-mark-circle' : 'dismiss-circle'"
                        :color="(item.aanvraag.niveau === 'LANDELIJK' ? uitkomst.heeft_recht_landelijk : uitkomst.heeft_recht_decentraal) ? 'success' : 'critical'"
                        size="20"
                      ></nldd-icon-cell>
                      <nldd-spacer-cell size="8"></nldd-spacer-cell>
                      <nldd-text-cell
                        :text="item.aanvraag.niveau === 'LANDELIJK' ? 'Recht op subsidie landelijk (art. 6)' : 'Recht op subsidie decentraal (art. 7)'"
                      ></nldd-text-cell>
                    </nldd-list-item>
                    <nldd-list-item size="sm">
                      <nldd-icon-cell icon="euro-sign" size="20" color="secondary"></nldd-icon-cell>
                      <nldd-spacer-cell size="8"></nldd-spacer-cell>
                      <nldd-text-cell
                        text="Betaalopdracht (art. 16)"
                        :supporting-text="uitkomst.betaalopdracht_vereist ? euro(uitkomst.betaalopdracht_bedrag) : 'Niet van toepassing'"
                      ></nldd-text-cell>
                    </nldd-list-item>
                    <nldd-list-item size="sm">
                      <nldd-icon-cell icon="clock" size="20" color="secondary"></nldd-icon-cell>
                      <nldd-spacer-cell size="8"></nldd-spacer-cell>
                      <nldd-text-cell
                        text="Bezwaartermijn (AWB 6:7)"
                        :supporting-text="`${uitkomst.bezwaartermijn_weken} weken na bekendmaking`"
                      ></nldd-text-cell>
                    </nldd-list-item>
                  </nldd-list>
                </nldd-container>
              </nldd-box>
              <nldd-spacer size="24"></nldd-spacer>
              <nldd-button
                variant="primary"
                :text="uitkomst.subsidie_toegekend ? 'Besluit vaststellen: toekennen' : 'Besluit vaststellen: afwijzen'"
                start-icon="check-mark"
                :disabled="bezig || undefined"
                @click="stelVast"
              ></nldd-button>
            </template>

            <!-- Besloten: bekendmaken of bezwaarfase -->
            <template v-else-if="item.besluit">
              <nldd-title size="4"><h3>Besluit</h3></nldd-title>
              <nldd-spacer size="12"></nldd-spacer>
              <NBanner
                :variant="item.besluit.subsidie_toegekend ? 'success' : 'critical'"
                :text="item.besluit.subsidie_toegekend
                  ? `Toegekend: ${euro(item.besluit.subsidiebedrag)}`
                  : 'Afgewezen'"
                :supporting-text="item.besluit.motivering"
              />
              <nldd-spacer size="16"></nldd-spacer>
              <LifecycleTimeline :aanvraag="item.aanvraag" :besluit="item.besluit" />
              <nldd-spacer size="16"></nldd-spacer>
              <nldd-button
                v-if="item.aanvraag.status === 'BESLUIT'"
                variant="primary"
                text="Besluit bekendmaken"
                start-icon="paper-plane"
                :disabled="bezig || undefined"
                @click="maakBekend"
              ></nldd-button>
              <nldd-rich-text v-else-if="item.besluit.bezwaartermijn_einddatum">
                <p>
                  Bekendgemaakt op {{ datum(item.besluit.bekendmaking_datum) }}.
                  De bezwaartermijn loopt tot en met
                  {{ datum(item.besluit.bezwaartermijn_einddatum) }}.
                </p>
              </nldd-rich-text>
            </template>
          </div>
        </nldd-one-half-one-half-section>

        <template v-if="fout">
          <nldd-spacer size="16"></nldd-spacer>
          <NBanner variant="critical" text="Er ging iets mis" :supporting-text="fout" />
        </template>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
