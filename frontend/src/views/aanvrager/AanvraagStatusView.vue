<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import LifecycleTimeline from '../../components/LifecycleTimeline.vue';
import { api } from '../../api.js';
import { euro, datum, onderdelen } from '../../format.js';

const route = useRoute();
const router = useRouter();
const item = ref(null);
const fout = ref('');

const componenten = computed(() => item.value?.besluit?.componenten ?? []);
const toegekendeComponenten = computed(() => componenten.value.filter((c) => c.toegekend));

function componentLabel(c) {
  if (c.soort === 'LANDELIJK') return 'Landelijke subsidie';
  const orgaan = {
    GEMEENTERAAD: 'Gemeenteraad',
    PROVINCIALE_STATEN: 'Provinciale staten',
    EILANDSRAAD: 'Eilandsraad',
    WATERSCHAP: 'Waterschap',
  }[c.orgaan] ?? c.orgaan;
  return `${orgaan} ${c.gebied ?? ''}`;
}

// De vier delen van art. 14 voor de landelijke component (indien toegekend).
const landelijkeDelen = computed(() => {
  const c = componenten.value.find((x) => x.soort === 'LANDELIJK' && x.delen);
  if (!c) return null;
  return [
    { label: 'Politieke partij (onderdeel a)', bedrag: c.delen.partij },
    { label: 'Politiek-wetenschappelijk instituut (b)', bedrag: c.delen.wetenschappelijk_instituut },
    { label: 'Politieke jongerenorganisatie (c)', bedrag: c.delen.jongerenorganisatie },
    { label: 'Instelling buitenlandse activiteiten (d)', bedrag: c.delen.buitenland },
  ].filter((d) => d.bedrag > 0);
});

onMounted(async () => {
  try {
    item.value = await api.mijnAanvraag(route.params.id);
  } catch (e) {
    fout.value = e.message;
  }
});
</script>

<template>
  <nldd-page>
    <PortalHeader
      slot="header"
      subtitle="Subsidieportaal politieke partijen"
      portal="aanvrager"
      :items="[{ text: 'Mijn aanvragen', to: '/' }, { text: 'Nieuwe aanvraag', to: '/nieuw' }, { text: 'Mijn organisatie', to: '/organisatie' }]"
    />

    <nldd-simple-section v-if="fout" width="560px">
      <NBanner variant="critical" text="Aanvraag niet gevonden" :supporting-text="fout" />
    </nldd-simple-section>

    <template v-else-if="item">
      <nldd-simple-section width="820px">
        <nldd-button
          variant="neutral-transparent"
          size="sm"
          text="Terug naar uw aanvragen"
          start-icon="chevron-left"
          @click="router.push('/')"
        ></nldd-button>
        <nldd-spacer size="16"></nldd-spacer>

        <nldd-title size="2">
          <span slot="overline">{{ item.aanvraag.partij_naam }}</span>
          <h2>Jaaraanvraag {{ item.aanvraag.subsidiejaar }} · {{ onderdelen(item.aanvraag.componenten.length) }}</h2>
        </nldd-title>
        <nldd-spacer size="24"></nldd-spacer>

        <NBanner
          v-if="item.besluit"
          :variant="item.besluit.subsidie_toegekend ? 'success' : 'critical'"
          :text="item.besluit.subsidie_toegekend
            ? `Subsidie toegekend: ${euro(item.besluit.subsidiebedrag)}`
            : 'Uw aanvraag is afgewezen'"
          :supporting-text="item.besluit.motivering"
        />
        <NBanner
          v-else
          variant="accent"
          text="Uw aanvraag is in behandeling"
          supporting-text="De Napp toetst uw aanvraag aan de Wet op de politieke partijen."
        />

        <!-- Het voorschot dat uit het besluit voortvloeit (art. 16/17 Wpp). -->
        <template v-if="item.betaalopdracht">
          <nldd-spacer size="12"></nldd-spacer>
          <NBanner
            v-if="item.betaalopdracht.status === 'AANGEHOUDEN'"
            variant="warning"
            :text="`Uitbetaling van het voorschot (${euro(item.betaalopdracht.bedrag)}) is aangehouden`"
            supporting-text="Er is nog geen rekeningnummer van uw rechtspersoon bekend (artikel 27 Wpp). Zodra het bestuur een rekening opgeeft bij Mijn organisatie, wordt de uitbetaling klaargezet."
          />
          <NBanner
            v-else-if="item.betaalopdracht.status === 'UITBETAALD'"
            variant="success"
            :text="`Het voorschot van ${euro(item.betaalopdracht.bedrag)} is uitbetaald`"
            :supporting-text="`Overgemaakt naar ${item.betaalopdracht.iban} op ${item.betaalopdracht.uitgevoerd_at}.`"
          />
          <NBanner
            v-else
            variant="accent"
            :text="`Het voorschot van ${euro(item.betaalopdracht.bedrag)} staat klaar voor uitbetaling`"
            :supporting-text="item.betaalopdracht.betaaltermijn_einddatum
              ? `Uitbetaling op ${item.betaalopdracht.iban}, uiterlijk ${datum(item.betaalopdracht.betaaltermijn_einddatum)} (AWB 4:87).`
              : `Uitbetaling op ${item.betaalopdracht.iban}.`"
          />
        </template>
        <nldd-spacer size="32"></nldd-spacer>

        <template v-if="item.besluit">
          <nldd-title size="4"><h3>Specificatie per onderdeel</h3></nldd-title>
          <nldd-spacer size="12"></nldd-spacer>
          <nldd-table
            columns="minmax(220px,1fr) 110px 120px 150px"
            accessible-label="Specificatie van het besluit"
          >
            <nldd-table-row slot="header">
              <nldd-text-cell text="Onderdeel"></nldd-text-cell>
              <nldd-text-cell text="Zetels" horizontal-alignment="right"></nldd-text-cell>
              <nldd-text-cell text="Besluit"></nldd-text-cell>
              <nldd-text-cell text="Bedrag" horizontal-alignment="right"></nldd-text-cell>
            </nldd-table-row>
            <nldd-table-row v-for="c in componenten" :key="c.key">
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

          <template v-if="landelijkeDelen">
            <nldd-spacer size="16"></nldd-spacer>
            <nldd-title size="5"><h4>Landelijke subsidie in delen (art. 14)</h4></nldd-title>
            <nldd-spacer size="8"></nldd-spacer>
            <nldd-list variant="box">
              <nldd-list-item v-for="d in landelijkeDelen" :key="d.label" size="sm">
                <nldd-text-cell :text="d.label" color="secondary"></nldd-text-cell>
                <nldd-text-cell :text="euro(d.bedrag)" horizontal-alignment="right"></nldd-text-cell>
              </nldd-list-item>
            </nldd-list>
          </template>
          <nldd-spacer size="32"></nldd-spacer>
        </template>

        <nldd-title size="4"><h3>Verloop van uw aanvraag</h3></nldd-title>
        <nldd-spacer size="12"></nldd-spacer>
        <LifecycleTimeline :aanvraag="item.aanvraag" :besluit="item.besluit" />

        <template v-if="item.besluit?.bezwaartermijn_einddatum">
          <nldd-spacer size="24"></nldd-spacer>
          <nldd-box>
            <nldd-container padding="16">
              <nldd-rich-text>
                <p>
                  Bent u het niet eens met dit besluit? U kunt tot en met
                  <strong>{{ datum(item.besluit.bezwaartermijn_einddatum) }}</strong>
                  bezwaar maken bij de Nederlandse autoriteit politieke partijen
                  (artikel 6:4 Algemene wet bestuursrecht).
                </p>
              </nldd-rich-text>
            </nldd-container>
          </nldd-box>
        </template>
      </nldd-simple-section>
    </template>
  </nldd-page>
</template>
