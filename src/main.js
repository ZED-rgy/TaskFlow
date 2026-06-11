import { createApp } from 'vue'
import App from './App.vue'
import Widget from './Widget.vue'
import './style.css'

const params = new URLSearchParams(window.location.search)
if (params.has('widget')) {
  document.documentElement.classList.add('widget-mode')
  document.body.classList.add('widget-mode')
}
createApp(params.has('widget') ? Widget : App).mount('#app')
