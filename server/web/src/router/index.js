import { createRouter, createWebHistory } from 'vue-router'
import Dashboard from '../views/Dashboard.vue'
import Listeners from '../views/Listeners.vue'
import Payload from '../views/Payload.vue'
import FileDrop from '../views/FileDrop.vue'
import Terminal from '../views/Terminal.vue'

const routes = [
  {
    path: '/',
    name: 'Dashboard',
    component: Dashboard
  },
  {
    path: '/listeners',
    name: 'Listeners',
    component: Listeners
  },
  {
    path: '/payload',
    name: 'Payload',
    component: Payload
  },
  {
    path: '/file-drop',
    name: 'FileDrop',
    component: FileDrop
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: Terminal
  }
]

export const router = createRouter({
  history: createWebHistory(),
  routes
})