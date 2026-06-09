<script setup>
import { computed } from 'vue';
import { datum } from '../format.js';

const props = defineProps({
  aanvraag: { type: Object, required: true },
  besluit: { type: Object, default: null },
});

// AWB-procedure (RFC-008): AANVRAAG → BEHANDELING → BESLUIT → BEKENDMAKING → BEZWAAR
const stappen = computed(() => {
  const status = props.aanvraag.status;
  const b = props.besluit;
  const bereikt = {
    AANVRAAG: true,
    BEHANDELING: true,
    BESLUIT: status === 'BESLUIT' || status === 'BEZWAAR',
    BEKENDMAKING: status === 'BEZWAAR',
    BEZWAAR: status === 'BEZWAAR',
  };
  return [
    {
      key: 'AANVRAAG',
      titel: 'Aanvraag ingediend',
      detail: `${datum(props.aanvraag.aanvraag_datum)} (AWB 4:1)`,
      bereikt: bereikt.AANVRAAG,
    },
    {
      key: 'BEHANDELING',
      titel: 'In behandeling bij de Napp',
      detail: 'De Napp toetst de aanvraag aan de wet (AWB 3:2)',
      bereikt: bereikt.BEHANDELING,
    },
    {
      key: 'BESLUIT',
      titel: b
        ? b.subsidie_toegekend
          ? 'Besluit: subsidie toegekend'
          : 'Besluit: aanvraag afgewezen'
        : 'Besluit',
      detail: b ? `${datum(b.besluit_datum)} (AWB 1:3)` : 'Volgt na de behandeling',
      bereikt: bereikt.BESLUIT,
    },
    {
      key: 'BEKENDMAKING',
      titel: 'Bekendmaking',
      detail: b?.bekendmaking_datum
        ? `${datum(b.bekendmaking_datum)} (AWB 3:41)`
        : 'Het besluit wordt aan u bekendgemaakt',
      bereikt: bereikt.BEKENDMAKING,
    },
    {
      key: 'BEZWAAR',
      titel: 'Bezwaartermijn',
      detail: b?.bezwaartermijn_einddatum
        ? `Zes weken, tot en met ${datum(b.bezwaartermijn_einddatum)} (AWB 6:7)`
        : 'Zes weken vanaf de dag na bekendmaking (AWB 6:7)',
      bereikt: bereikt.BEZWAAR,
    },
  ];
});
</script>

<template>
  <nldd-list variant="simple">
    <nldd-list-item v-for="(stap, i) in stappen" :key="stap.key" size="md">
      <nldd-timeline-track-cell
        :step="stap.bereikt ? 'past' : 'future'"
        :child="i === 0 ? 'first' : i === stappen.length - 1 ? 'last' : 'between'"
      ></nldd-timeline-track-cell>
      <nldd-spacer-cell size="12"></nldd-spacer-cell>
      <nldd-text-cell
        :text="stap.titel"
        :supporting-text="stap.detail"
        :color="stap.bereikt ? 'default' : 'secondary'"
      ></nldd-text-cell>
    </nldd-list-item>
  </nldd-list>
</template>
