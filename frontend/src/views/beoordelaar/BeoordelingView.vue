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

const eigen = computed(() => item.value?.aanvraag.parameters ?? {});

const verklaringen = computed(() => {
  const p = eigen.value;
  return [
    { label: 'Geen anonieme giften', ok: p.ontvangt_anonieme_giften === false },
    { label: 'Geen giften van niet-ingezetenen', ok: p.ontvangt_giften_niet_ingezetenen === false },
    { label: 'Meldplicht giften ≥ € 10.000 nageleefd', ok: p.voldoet_aan_meldplicht_giften === true },
    { label: 'Financiën openbaar op website', ok: p.financien_openbaar_op_website === true },
  ];
});

// De specificatie: uit het besluit (na vaststelling) of de proefberekening.
const specificatie = computed(
  () => item.value?.besluit?.componenten ?? uitkomst.value?.componenten ?? [],
);

const samenvattingPerGroep = computed(() => {
  const groepen = new Map();
  for (const c of specificatie.value) {
    const naam = c.soort === 'LANDELIJK'
      ? 'Landelijk'
      : { GEMEENTERAAD: 'Gemeenteraden', PROVINCIALE_STATEN: 'Provinciale staten', WATERSCHAP: 'Waterschappen' }[c.orgaan] ?? c.orgaan;
    const g = groepen.get(naam) ?? { naam, aantal: 0, toegekend: 0, bedrag: 0 };
    g.aantal += 1;
    if (c.toegekend) {
      g.toegekend += 1;
      g.bedrag += c.bedrag;
    }
    groepen.set(naam, g);
  }
  return [...groepen.values()];
});

function componentLabel(c) {
  if (c.soort === 'LANDELIJK') return 'Landelijke subsidie';
  const orgaan = {
    GEMEENTERAAD: 'Gemeenteraad',
    PROVINCIALE_STATEN: 'Provinciale staten',
    WATERSCHAP: 'Waterschap',
  }[c.orgaan] ?? c.orgaan;
  return `${orgaan} ${c.gebied ?? ''}`;
}

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
      portal="beoordelaar"
      :items="[{ text: 'Werkvoorraad', to: '/' }, { text: 'Scenario’s', to: '/scenarios' }]"
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
          <span slot="overline">Jaaraanvraag {{ item.aanvraag.subsidiejaar }} · KVK {{ item.aanvraag.kvk_nummer }}</span>
          <h2>{{ item.aanvraag.partij_naam }}</h2>
        </nldd-title>
        <nldd-spacer size="24"></nldd-spacer>

        <nldd-one-half-one-half-section padding-block="0">
          <div slot="left">
            <nldd-title size="4"><h3>Eigen opgaven</h3></nldd-title>
            <nldd-spacer size="12"></nldd-spacer>
            <nldd-list variant="box">
              <nldd-list-item size="sm">
                <nldd-text-cell text="Onderdelen in deze aanvraag" color="secondary"></nldd-text-cell>
                <nldd-text-cell :text="String(item.aanvraag.componenten.length)" horizontal-alignment="right"></nldd-text-cell>
              </nldd-list-item>
              <nldd-list-item size="sm">
                <nldd-text-cell text="Betalende leden · eigen opgave" color="secondary"></nldd-text-cell>
                <nldd-text-cell :text="String(eigen.aantal_betalende_leden ?? 0)" horizontal-alignment="right"></nldd-text-cell>
              </nldd-list-item>
              <nldd-list-item v-if="item.aanvraag.beslistermijn_einddatum" size="sm">
                <nldd-text-cell text="Beslistermijn · AWB 4:13" color="secondary"></nldd-text-cell>
                <nldd-text-cell :text="datum(item.aanvraag.beslistermijn_einddatum)" horizontal-alignment="right"></nldd-text-cell>
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

            <template v-if="item.besluit">
              <nldd-spacer size="24"></nldd-spacer>
              <nldd-title size="4"><h3>Verloop</h3></nldd-title>
              <nldd-spacer size="12"></nldd-spacer>
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

          <div slot="right">
            <nldd-title size="4">
              <h3>{{ item.besluit ? 'Besluit' : 'Uitkomst volgens de wet' }}</h3>
            </nldd-title>
            <nldd-spacer size="12"></nldd-spacer>

            <template v-if="item.besluit || uitkomst">
              <NBanner
                :variant="(item.besluit ?? uitkomst).subsidie_toegekend ? 'success' : 'critical'"
                :text="(item.besluit ?? uitkomst).subsidie_toegekend
                  ? `${item.besluit ? 'Toegekend' : 'Toekenning'}: ${euro((item.besluit ?? uitkomst).subsidiebedrag)}`
                  : 'Afwijzing'"
                :supporting-text="(item.besluit ?? uitkomst).motivering"
              />
              <nldd-spacer size="16"></nldd-spacer>

              <nldd-list variant="box">
                <nldd-list-item v-for="g in samenvattingPerGroep" :key="g.naam" size="sm">
                  <nldd-text-cell
                    :text="g.naam"
                    :supporting-text="`${g.toegekend} van ${g.aantal} toegekend`"
                  ></nldd-text-cell>
                  <nldd-text-cell :text="euro(g.bedrag)" horizontal-alignment="right"></nldd-text-cell>
                </nldd-list-item>
              </nldd-list>
              <nldd-spacer size="16"></nldd-spacer>

              <nldd-button
                v-if="item.aanvraag.status === 'BEHANDELING' && uitkomst"
                variant="primary"
                :text="uitkomst.subsidie_toegekend
                  ? `Besluit vaststellen: toekennen (${euro(uitkomst.subsidiebedrag)})`
                  : 'Besluit vaststellen: afwijzen'"
                start-icon="check-mark"
                :disabled="bezig || undefined"
                @click="stelVast"
              ></nldd-button>

              <nldd-spacer size="24"></nldd-spacer>
              <nldd-title size="5"><h4>Specificatie per onderdeel</h4></nldd-title>
              <nldd-spacer size="8"></nldd-spacer>
              <nldd-table
                columns="minmax(180px,1fr) 70px 110px 130px"
                accessible-label="Specificatie per onderdeel"
              >
                <nldd-table-row slot="header">
                  <nldd-text-cell text="Onderdeel"></nldd-text-cell>
                  <nldd-text-cell text="Zetels" horizontal-alignment="right"></nldd-text-cell>
                  <nldd-text-cell text="Besluit"></nldd-text-cell>
                  <nldd-text-cell text="Bedrag" horizontal-alignment="right"></nldd-text-cell>
                </nldd-table-row>
                <nldd-table-row v-for="c in specificatie" :key="c.key">
                  <nldd-text-cell :text="componentLabel(c)"></nldd-text-cell>
                  <nldd-text-cell :text="String(c.zetels)" horizontal-alignment="right"></nldd-text-cell>
                  <nldd-cell>
                    <nldd-tag
                      :color="c.toegekend ? 'success' : 'critical'"
                      :text="c.toegekend ? 'Toegekend' : 'Afgewezen'"
                    ></nldd-tag>
                  </nldd-cell>
                  <nldd-text-cell
                    :text="c.toegekend ? euro(c.bedrag) : '—'"
                    horizontal-alignment="right"
                  ></nldd-text-cell>
                </nldd-table-row>
              </nldd-table>
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
