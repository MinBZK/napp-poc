import '@nldd/design-system';
import '@nldd/design-system/styles';
import '../theme.css';
import { createApp } from 'vue';
import { createRouter, createWebHistory } from 'vue-router';
import App from './App.vue';
import AanvragerView from '../views/aanvrager/AanvragerView.vue';
import AanvraagNieuwView from '../views/aanvrager/AanvraagNieuwView.vue';
import AanvraagStatusView from '../views/aanvrager/AanvraagStatusView.vue';

const router = createRouter({
  history: createWebHistory('/aanvrager/'),
  routes: [
    { path: '/', component: AanvragerView },
    { path: '/nieuw', component: AanvraagNieuwView },
    { path: '/aanvraag/:id', component: AanvraagStatusView },
  ],
});

createApp(App).use(router).mount('#app');
