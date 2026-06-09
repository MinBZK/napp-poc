<script setup>
/**
 * Gedeelde kop voor alle drie de ingangen: rijkslogo-navigatiebalk
 * (nldd-top-navigation-bar) met portaal-specifieke menu-items.
 * Items met `to` navigeren binnen het portaal (SPA); items met `href`
 * verlaten het portaal (volledige paginalading).
 */
import { useRoute, useRouter } from 'vue-router';

const props = defineProps({
  subtitle: { type: String, required: true },
  items: { type: Array, default: () => [] }, // { text, to? , href?, icon? }
  utilityItems: { type: Array, default: () => [] },
});
const emit = defineEmits(['utility']);

const router = useRouter();
const route = useRoute();

function isCurrent(item) {
  if (!item.to) return false;
  if (item.to === '/') return route.path === '/';
  return route.path.startsWith(item.to);
}

function onSelect(event, item) {
  if (item.to) {
    event.preventDefault();
    router.push(item.to);
  }
}
</script>

<template>
  <nldd-skip-link text="Direct naar de inhoud">
    <nldd-top-navigation-bar
      logo-title="Napp"
      :logo-subtitle="subtitle"
      logo-href="/"
      website-href="/"
    >
      <nldd-menu-bar v-if="items.length" slot="global">
        <nldd-menu-bar-item
          v-for="item in items"
          :key="item.text"
          :text="item.text"
          :href="item.href ?? item.to"
          :icon="item.icon"
          :current="isCurrent(item) || undefined"
          @click="onSelect($event, item)"
        ></nldd-menu-bar-item>
      </nldd-menu-bar>
      <nldd-menu-bar v-if="utilityItems.length" slot="utility">
        <nldd-menu-bar-item
          v-for="item in utilityItems"
          :key="item.text"
          :text="item.text"
          :icon="item.icon"
          @click="emit('utility', item)"
        ></nldd-menu-bar-item>
      </nldd-menu-bar>
    </nldd-top-navigation-bar>
  </nldd-skip-link>
</template>
