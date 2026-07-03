// 主题配色定义与归一化。从 App.vue 抽出，均为纯数据 / 纯函数，无响应式依赖。

export const THEMES = [
  {
    id: 'morning',
    name: '晨雾',
    desc: '清爽浅色，适合白天长时间整理任务',
    swatches: ['#ECF1F6', '#FFFFFF', '#AE6A21', '#3B73A4'],
  },
  {
    id: 'midnight',
    name: '墨蓝',
    desc: '低亮度深色，适合晚上和专注时段',
    swatches: ['#11161D', '#171E27', '#DC9036', '#5189BB'],
  },
  {
    id: 'forest',
    name: '森林',
    desc: '偏自然的绿调，适合日程和生活任务',
    swatches: ['#EBF2EC', '#FCFEFB', '#5B7C3B', '#2E7864'],
  },
  {
    id: 'graphite',
    name: '石墨',
    desc: '克制中性灰，适合工作型任务管理',
    swatches: ['#E7EBF0', '#FCFDFE', '#516070', '#4E7A9C'],
  },
  {
    id: 'apricot',
    name: '暮杏',
    desc: '温暖柔和，适合低压的个人规划',
    swatches: ['#F5EDE5', '#FFFCF9', '#BC6A37', '#6E8C8A'],
  },
]

export const THEME_IDS = THEMES.map(item => item.id)

export function normalizeTheme(value) {
  if (value === 'light') return 'morning'
  if (value === 'dark') return 'midnight'
  return THEME_IDS.includes(value) ? value : 'morning'
}
