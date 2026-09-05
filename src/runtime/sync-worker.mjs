// Serialize the entire pass, including snapshot application and conflict dialogs.
// Realtime is an optional accelerator; polling starts even when it is unavailable.
export function createSyncWorker({ run, subscribe, onConnectionChange = () => {},
  schedule = setInterval, cancel = clearInterval, now = Date.now } = {}) {
  let active = true
  let timer = null
  let pass = null
  let connecting = null
  let unsubscribe = null
  let retryAt = 0
  const isActive = () => active

  function sync() {
    if (!active) return Promise.resolve()
    if (!pass) pass = Promise.resolve().then(() => active && run(isActive))
      .finally(() => { pass = null })
    return pass
  }

  function connect() {
    if (!active || connecting || unsubscribe || now() < retryAt) return
    connecting = Promise.resolve().then(() => {
      if (!active) return null
      return subscribe(() => { void sync() }, connected => {
        if (active) onConnectionChange(connected)
      })
    }).then(cleanup => {
      if (!active) { cleanup?.(); return }
      unsubscribe = cleanup
      onConnectionChange(true)
    }).catch(() => {
      if (active) onConnectionChange(false)
      retryAt = now() + 30_000
    }).finally(() => { connecting = null })
  }

  return {
    run: sync,
    start() {
      if (!active || timer !== null) return
      timer = schedule(() => { void sync(); connect() }, 5000)
      connect()
      void sync()
    },
    stop() {
      active = false
      if (timer !== null) cancel(timer)
      timer = null
      unsubscribe?.()
      unsubscribe = null
    },
  }
}
