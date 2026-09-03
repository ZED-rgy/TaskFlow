import { createApp } from 'vue'
import App from './App.vue'
import Widget from './Widget.vue'
import QuickAdd from './QuickAdd.vue'
import './style.css'

const params = new URLSearchParams(window.location.search)
const isDev = import.meta.env.DEV
const isAndroid = /Android/i.test(navigator.userAgent)
if (isAndroid) document.documentElement.classList.add('platform-android')
if (params.has('widget') || params.has('quickadd')) {
  document.documentElement.classList.add('widget-mode')
  document.body.classList.add('widget-mode')
}
async function bootstrap() {
  const Root = isDev && params.has('prototype')
    ? (await import('./Prototype.vue')).default
    : params.has('quickadd')
      ? QuickAdd
      : params.has('widget')
        ? Widget
        : App
  createApp(Root).mount('#app')
}

bootstrap()
