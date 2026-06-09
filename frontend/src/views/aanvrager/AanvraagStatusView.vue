<script setup>
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import PortalHeader from '../../components/PortalHeader.vue';
import NBanner from '../../components/NBanner.vue';
import LifecycleTimeline from '../../components/LifecycleTimeline.vue';
import { api } from '../../api.js';
import { euro, datum } from '../../format.js';

const route = useRoute();
const router = useRouter();
const item = ref(null);
const fout = ref('');

onMounted(async () => {
  try {
    item.value = await api.aanvraag(route.params.id);
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
      :items="[{ text: 'Mijn aanvragen', to: '/' }, { text: 'Nieuwe aanvraag', to: '/nieuw' }]"
    />

    <nldd-simple-section v-if="fout" width="560px">
      <NBanner variant="critical" text="Aanvraag niet gevonden" :supporting-text="fout" />
    </nldd-simple-section>

    <template v-else-if="item">
      <nldd-simple-section width="720px">
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
          <h2>
            {{ item.aanvraag.niveau === 'LANDELIJK' ? 'Aanvraag landelijke subsidie' : 'Aanvraag decentrale subsidie' }}
          </h2>
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
        <nldd-spacer size="32"></nldd-spacer>

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
