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
    component: Dashboard,
    alias: '/home/index.html'
  },
  {
    path: '/listeners',
    name: 'Listeners',
    component: Listeners,
    alias: '/home/listeners.html'
  },
  {
    path: '/payload',
    name: 'Payload',
    component: Payload,
    alias: '/home/payload.html'
  },
  {
    path: '/file-drop',
    name: 'FileDrop',
    component: FileDrop,
    alias: '/home/file_drop.html'
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: Terminal,
    alias: '/home/server_terminal.html'
  }
]

export const router = createRouter({
  history: createWebHistory(),
  routes
})