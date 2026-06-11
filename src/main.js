import { createApp } from 'vue'
import App from './App.vue'
import Widget from './Widget.vue'
import QuickAdd from './QuickAdd.vue'
import './style.css'

const params = new URLSearchParams(window.location.search)
if (params.has('widget') || params.has('quickadd')) {
  document.documentElement.classList.add('widget-mode')
  document.body.classList.add('widget-mode')
}
const Root = params.has('quickadd') ? QuickAdd : params.has('widget') ? Widget : App
createApp(Root).mount('#app')
