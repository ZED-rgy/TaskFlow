// 快速添加自然语言解析：从标题文本中识别日期、优先级、标签。
// 纯函数，无副作用；被 TaskList 添加栏和 QuickAdd 全局窗口共用。
//
// 支持语法（示例：「明天下午 交报告 #学校 !高」）：
//   日期    今天/明天/后天/大后天、N天后、周三/星期三/礼拜三、下周三、
//           7月15日、7月15号、7-15、7/15、2026-07-15
//   优先级  !高 !低 !h !l !!（高）、！高（全角）
//   标签    #标签名（可多个）

const WEEKDAY_MAP = { 一: 1, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6, 日: 0, 天: 0 }
const CN_NUM = { 一: 1, 两: 2, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9, 十: 10 }

function pad(n) {
  return String(n).padStart(2, '0')
}

function keyOf(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

function addDays(base, days) {
  const d = new Date(base.getTime())
  d.setDate(d.getDate() + days)
  return d
}

function clampMonthDay(year, month, day) {
  const lastDay = new Date(year, month, 0).getDate()
  return new Date(year, month - 1, Math.min(day, lastDay))
}

// 周X → 未来最近的那个周X（若今天就是周X，取今天）；
// 下周X → 下一个自然周（周一为一周起点）里的周X
function nextWeekday(base, weekday, forceNextWeek) {
  let d = addDays(base, (weekday - base.getDay() + 7) % 7)
  if (forceNextWeek) {
    const dow = base.getDay() || 7 // 周日按 7 算
    const startOfNextWeek = addDays(base, 8 - dow)
    while (d < startOfNextWeek) d = addDays(d, 7)
  }
  return d
}

// 日期规则表：[正则, 解析函数(match, base) => Date|null]
const DATE_RULES = [
  [/今天|今日/, (m, base) => base],
  [/明天|明日/, (m, base) => addDays(base, 1)],
  [/大后天/, (m, base) => addDays(base, 3)],
  [/后天/, (m, base) => addDays(base, 2)],
  [/(\d+|[一两二三四五六七八九十])\s*天后/, (m, base) => {
    const n = CN_NUM[m[1]] ?? parseInt(m[1], 10)
    return Number.isFinite(n) && n > 0 && n <= 365 ? addDays(base, n) : null
  }],
  [/(下+)?\s*(?:周|星期|礼拜)([一二三四五六日天])/, (m, base) => {
    const weekday = WEEKDAY_MAP[m[2]]
    let d = nextWeekday(base, weekday, Boolean(m[1]))
    // 「下下周X」再顺延一周
    if (m[1] && m[1].length > 1) d = addDays(d, 7 * (m[1].length - 1))
    return d
  }],
  [/(\d{4})[-/](\d{1,2})[-/](\d{1,2})/, m => {
    const [_, y, mo, day] = m
    if (+mo < 1 || +mo > 12 || +day < 1 || +day > 31) return null
    return clampMonthDay(+y, +mo, +day)
  }],
  [/(\d{1,2})月(\d{1,2})[日号]?/, (m, base) => {
    if (+m[1] < 1 || +m[1] > 12 || +m[2] < 1 || +m[2] > 31) return null
    let d = clampMonthDay(base.getFullYear(), +m[1], +m[2])
    if (keyOf(d) < keyOf(base)) d = clampMonthDay(base.getFullYear() + 1, +m[1], +m[2])
    return d
  }],
  [/(\d{1,2})[-/](\d{1,2})/, (m, base) => {
    if (+m[1] < 1 || +m[1] > 12 || +m[2] < 1 || +m[2] > 31) return null
    let d = clampMonthDay(base.getFullYear(), +m[1], +m[2])
    if (keyOf(d) < keyOf(base)) d = clampMonthDay(base.getFullYear() + 1, +m[1], +m[2])
    return d
  }],
]

// 词元需与上下文分隔（中文无 \b）：前面是开头/空白，或后面是空白/结尾，避免
// 「写周报」「后天气」这类误判——纯中文句内不解析，加空格或放句首/句尾即可触发。
function findBounded(text, pattern) {
  const re = new RegExp(pattern.source, 'g')
  let match
  while ((match = re.exec(text)) !== null) {
    const before = text[match.index - 1]
    const after = text[match.index + match[0].length]
    const beforeOk = match.index === 0 || /\s/.test(before)
    const afterOk = after === undefined || /\s/.test(after)
    if (beforeOk || afterOk) return match
    if (re.lastIndex === match.index) re.lastIndex += 1
  }
  return null
}

function stripToken(text, match) {
  return (text.slice(0, match.index) + ' ' + text.slice(match.index + match[0].length))
    .replace(/\s{2,}/g, ' ')
    .trim()
}

/**
 * @param {string} raw 用户输入
 * @param {string} todayKey 今天的 YYYY-MM-DD（本地时区）
 * @returns {{ title, dueDate, priority, tags, hits }}
 *   hits: [{ type: 'date'|'priority'|'tag', text, value }] 用于界面预览
 */
export function parseQuickInput(raw, todayKey) {
  let title = String(raw || '').trim()
  const hits = []
  const base = todayKey
    ? new Date(`${todayKey}T00:00:00`)
    : new Date(new Date().setHours(0, 0, 0, 0))

  // 1. 标签（# 后跟非空白，排除 # ! 本身）
  const tags = []
  title = title.replace(/(?:^|\s)#([^\s#!！]{1,40})/g, (whole, tag) => {
    tags.push(tag)
    hits.push({ type: 'tag', text: `#${tag}`, value: tag })
    return ' '
  }).replace(/\s{2,}/g, ' ').trim()

  // 2. 优先级
  let priority = null
  const prMatch = title.match(/(?:^|\s)(?:[!！](高|低|中|h|l|m|high|low)|!!|！！)(?=\s|$)/i)
  if (prMatch) {
    const token = (prMatch[1] || '').toLowerCase()
    priority = prMatch[0].includes('!!') || prMatch[0].includes('！！') || token === '高' || token === 'h' || token === 'high'
      ? 'high'
      : (token === '低' || token === 'l' || token === 'low' ? 'low' : 'normal')
    hits.push({ type: 'priority', text: prMatch[0].trim(), value: priority })
    title = stripToken(title, { index: prMatch.index, 0: prMatch[0] })
  }

  // 3. 日期（按规则表顺序，取第一个命中的）
  let dueDate = null
  for (const [pattern, resolve] of DATE_RULES) {
    const match = findBounded(title, pattern)
    if (!match) continue
    const parsed = resolve(match, base)
    if (!parsed) continue
    dueDate = keyOf(parsed)
    hits.push({ type: 'date', text: match[0].trim(), value: dueDate })
    title = stripToken(title, match)
    break
  }

  return { title, dueDate, priority, tags, hits }
}


// 供界面显示：把 YYYY-MM-DD 转成友好文案（今天/明天/周X/M月D日）
export function friendlyDate(dateKey, todayKey) {
  if (!dateKey) return ''
  const target = new Date(`${dateKey}T00:00:00`)
  const base = new Date(`${todayKey}T00:00:00`)
  const diff = Math.round((target - base) / 86400000)
  if (diff === 0) return '今天'
  if (diff === 1) return '明天'
  if (diff === 2) return '后天'
  if (diff > 2 && diff < 7) {
    return '周' + '日一二三四五六'[target.getDay()]
  }
  return `${target.getMonth() + 1}月${target.getDate()}日`
}
