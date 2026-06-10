<script setup>
/**
 * Result rows of the (mocked) Handelsregister check stored with a claim:
 * registration, rechtsvorm, SBI code and name match, each with a green
 * (in orde) or red (afwijkend) tag. Shared between the aanvrager claim
 * block and the beoordelaar claims section.
 */
import { computed } from 'vue';

const props = defineProps({
  toets: { type: Object, default: () => ({}) },
});

const regels = computed(() => {
  const t = props.toets ?? {};
  return [
    {
      label: 'Inschrijving Handelsregister',
      waarde: t.gevonden
        ? (t.statutaire_naam ?? 'Gevonden')
        : 'Geen inschrijving gevonden',
      ok: !!t.gevonden,
    },
    {
      label: 'Rechtsvorm (Kieswet G-1: vereniging met volledige rechtsbevoegdheid)',
      waarde: t.rechtsvorm ?? 'Onbekend',
      ok: t.rechtsvorm === 'Vereniging met volledige rechtsbevoegdheid',
    },
    {
      label: 'SBI-code (politieke organisaties: 94.92)',
      waarde: t.sbi_code
        ? `${t.sbi_code}${t.sbi_omschrijving ? ` · ${t.sbi_omschrijving}` : ''}`
        : 'Onbekend',
      ok: t.sbi_code === '94.92',
    },
    {
      label: 'Statutaire naam en aanduiding',
      waarde: t.naam_match ? 'Komen overeen' : 'Wijken af',
      ok: !!t.naam_match,
    },
  ];
});
</script>

<template>
  <nldd-list variant="box">
    <nldd-list-item v-for="regel in regels" :key="regel.label" size="sm">
      <nldd-text-cell :text="regel.label" :supporting-text="regel.waarde"></nldd-text-cell>
      <nldd-spacer-cell size="8"></nldd-spacer-cell>
      <nldd-cell width="fit-content">
        <nldd-tag
          :color="regel.ok ? 'success' : 'critical'"
          :text="regel.ok ? 'In orde' : 'Afwijkend'"
        ></nldd-tag>
      </nldd-cell>
    </nldd-list-item>
  </nldd-list>
</template>
