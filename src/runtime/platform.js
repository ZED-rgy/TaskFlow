// 运行平台判定。main.js 在启动时给 <html> 加 platform-android 类，
// 这里提供同一判定的 JS 版本，供组件用 v-if 决定渲染哪套布局，
// 避免两套布局同时渲染再靠 CSS 隐藏其中一套。
export const isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent)
