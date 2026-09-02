const MAX_PUSH_BATCH = 50

function maxCursor(events) {
  const values = events
    .map(event => Number(event?.seq))
    .filter(value => Number.isSafeInteger(value) && value >= 0)
  return values.length ? Math.max(...values) : null
}

/**
 * Coordinates one safe sync pass without deciding how remote snapshots are applied.
 * The caller must apply returned remoteEvents successfully before committing nextCursor.
 */
export function createSyncEngine({ localApi, repository, onStateChange } = {}) {
  let running = false

  const emit = state => onStateChange?.(state)

  return {
    get running() {
      return running
    },

    async syncOnce() {
      if (running) return { kind: 'busy', pushed: 0, remoteEvents: [] }
      if (!localApi || !repository?.enabled) {
        return { kind: 'disabled', pushed: 0, remoteEvents: [] }
      }

      running = true
      emit({ kind: 'syncing' })
      try {
        const status = await localApi.getSyncStatus()
        if (!status?.workspaceId) {
          const result = { kind: 'unbound', pushed: 0, remoteEvents: [] }
          emit(result)
          return result
        }

        // Pull first from the last committed cursor. Remote events are not acknowledged
        // here because the caller may still reject a snapshot during conflict handling.
        const remoteEvents = await repository.pullChanges({
          workspaceId: status.workspaceId,
          cursor: status.cursor,
        })

        const outbox = await localApi.getSyncOutbox()
        const pending = Array.isArray(outbox?.outbox) ? outbox.outbox.slice(0, MAX_PUSH_BATCH) : []
        const pushedIds = []
        for (const operation of pending) {
          await repository.pushOperation({
            workspaceId: status.workspaceId,
            deviceId: status.deviceId,
            operation,
          })
          pushedIds.push(operation.operationId)
        }
        if (pushedIds.length) await localApi.acknowledgeSync(pushedIds)

        const result = {
          kind: 'ready',
          pushed: pushedIds.length,
          remoteEvents,
          nextCursor: maxCursor(remoteEvents),
        }
        emit(result)
        return result
      } catch (error) {
        const result = { kind: 'error', pushed: 0, remoteEvents: [], error }
        emit(result)
        return result
      } finally {
        running = false
      }
    },

    async commitRemoteCursor(cursor) {
      if (!localApi || cursor === null || cursor === undefined) return null
      return localApi.acknowledgeSync([], String(cursor))
    },
  }
}
