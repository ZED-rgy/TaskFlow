// 字体相关的常量与纯函数。从 App.vue 抽出，无响应式依赖，便于单独维护和测试。

export const FONT_SIZES = {
  small: { size: '12px', scale: 0.92 },
  medium: { size: '13px', scale: 1 },
  large: { size: '15px', scale: 1.14 },
}

export const COMMON_CHINESE_FONTS = [
  { css: 'Microsoft YaHei UI', display: '微软雅黑 UI', search: 'Microsoft YaHei UI 微软雅黑 微软雅黑UI yahei' },
  { css: 'Microsoft YaHei', display: '微软雅黑', search: 'Microsoft YaHei 微软雅黑 yahei' },
  { css: 'DengXian', display: '等线', search: 'DengXian 等线 dengxian' },
  { css: 'SimSun', display: '宋体', search: 'SimSun 宋体 songti song' },
  { css: 'NSimSun', display: '新宋体', search: 'NSimSun 新宋体 songti song' },
  { css: 'SimHei', display: '黑体', search: 'SimHei 黑体 heiti hei' },
  { css: 'KaiTi', display: '楷体', search: 'KaiTi 楷体 kaiti kai' },
  { css: 'FangSong', display: '仿宋', search: 'FangSong 仿宋 fangsong song' },
  { css: 'YouYuan', display: '幼圆', search: 'YouYuan 幼圆 youyuan yuan' },
  { css: 'FZShuTi', display: '方正舒体', search: 'FZShuTi 方正舒体 fzshuti fangzheng shu' },
  { css: 'FZYaoTi', display: '方正姚体', search: 'FZYaoTi 方正姚体 fzyaoti fangzheng yao' },
  { css: 'LiSu', display: '隶书', search: 'LiSu 隶书 lishu li' },
  { css: 'STSong', display: '华文宋体', search: 'STSong 华文宋体 songti song' },
  { css: 'STZhongsong', display: '华文中宋', search: 'STZhongsong 华文中宋 songti song' },
  { css: 'STKaiti', display: '华文楷体', search: 'STKaiti 华文楷体 kaiti kai' },
  { css: 'STFangsong', display: '华文仿宋', search: 'STFangsong 华文仿宋 fangsong song' },
  { css: 'STXihei', display: '华文细黑', search: 'STXihei 华文细黑 heiti hei' },
  { css: 'STXingkai', display: '华文行楷', search: 'STXingkai 华文行楷 xingkai xing 星 行 楷' },
  { css: 'STXinwei', display: '华文新魏', search: 'STXinwei 华文新魏 xinwei wei' },
  { css: 'STLiti', display: '华文隶书', search: 'STLiti 华文隶书 lishu li' },
  { css: 'STCaiyun', display: '华文彩云', search: 'STCaiyun 华文彩云 caiyun yun' },
  { css: 'STHupo', display: '华文琥珀', search: 'STHupo 华文琥珀 hupo' },
  { css: 'Noto Sans SC', display: '思源黑体 / Noto Sans SC', search: 'Noto Sans SC 思源黑体 noto source han sans hei' },
  { css: 'Noto Serif SC', display: '思源宋体 / Noto Serif SC', search: 'Noto Serif SC 思源宋体 noto source han serif song' },
  { css: 'Source Han Serif SC', display: '思源宋体 / Source Han Serif', search: 'Source Han Serif SC 思源宋体 source han serif song' },
  { css: 'HYZhongHeiTi', display: '汉仪中黑体', search: 'HYZhongHeiTi 汉仪中黑体 hanyi zhonghei hei' },
]

export const LATIN_FONTS = [
  'Segoe UI',
  'Arial',
  'Calibri',
  'Consolas',
].map(name => ({ css: name, display: name, search: name }))

export const FALLBACK_FONTS = [...COMMON_CHINESE_FONTS, ...LATIN_FONTS]

export const CHINESE_SEARCH_ALIASES = {
  微: 'wei', 软: 'ruan', 雅: 'ya', 黑: 'hei', 宋: 'song', 楷: 'kai', 仿: 'fang',
  等: 'deng', 线: 'xian', 圆: 'yuan', 幼: 'you', 隶: 'li', 书: 'shu',
  华: 'hua', 文: 'wen', 行: 'xing', 星: 'xing', 新: 'xin', 魏: 'wei',
  彩: 'cai', 云: 'yun', 琥: 'hu', 珀: 'po', 思: 'si', 源: 'yuan',
  汉: 'han', 仪: 'yi', 中: 'zhong', 方: 'fang', 正: 'zheng', 舒: 'shu', 姚: 'yao',
}

export function normalizeFontName(value) {
  return String(value || '')
    .replace(/\s*\((TrueType|OpenType|All res)\)\s*/gi, '')
    .trim()
}

export function fontSearchText(font) {
  return [
    font.css,
    font.display,
    font.search,
    font.file,
  ].filter(Boolean).join(' ').toLowerCase()
}

export function expandFontQuery(query) {
  const lower = query.trim().toLowerCase()
  const pinyin = [...query].map(char => CHINESE_SEARCH_ALIASES[char] || '').filter(Boolean).join(' ')
  return [lower, pinyin].filter(Boolean)
}

export function mergeFonts(fonts = []) {
  const byCss = new Map()
  for (const font of [...COMMON_CHINESE_FONTS, ...fonts, ...LATIN_FONTS]) {
    const css = normalizeFontName(font.css || font.display)
    if (!css) continue
    const known = COMMON_CHINESE_FONTS.find(item =>
      item.css.toLowerCase() === css.toLowerCase() ||
      fontSearchText(item).includes(css.toLowerCase())
    )
    const item = {
      css: known?.css || css,
      display: known?.display || normalizeFontName(font.display || css),
      search: [known?.search, font.search, font.display, font.css, font.file].filter(Boolean).join(' '),
      file: font.file || '',
    }
    const key = item.css.toLowerCase()
    if (!byCss.has(key)) byCss.set(key, item)
  }
  return [...byCss.values()].sort((a, b) => {
    const ac = /[一-鿿]/.test(a.display) ? 0 : 1
    const bc = /[一-鿿]/.test(b.display) ? 0 : 1
    return ac - bc || a.display.localeCompare(b.display, 'zh-Hans-CN')
  })
}

export function fontStack(name, fallback = 'system-ui, sans-serif') {
  if (!name) return ''
  return `"${String(name).replace(/"/g, '\\"')}", ${fallback}`
}
