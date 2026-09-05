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
  let pendingRemoteCursor = null
  let stopped = false
  const checkActive = () => {
    if (stopped) throw Object.assign(new Error('同步已停止'), { code: 'SYNC_STOPPED' })
  }

  const emit = state => onStateChange?.(state)

  return {
    stop() { stopped = true; pendingRemoteCursor = null },
    get running() {
      return running
    },

    async syncOnce() {
      if (stopped) return { kind: 'stopped', pushed: 0, remoteEvents: [] }
      if (running) return { kind: 'busy', pushed: 0, remoteEvents: [] }
      if (!localApi || !repository?.enabled) {
        return { kind: 'disabled', pushed: 0, remoteEvents: [] }
      }

      running = true
      emit({ kind: 'syncing' })
      try {
        const status = await localApi.getSyncStatus()
        checkActive()
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
        checkActive()

        const outbox = await localApi.getSyncOutbox()
        checkActive()
        const pending = Array.isArray(outbox?.outbox) ? outbox.outbox.slice(0, MAX_PUSH_BATCH) : []
        const foreignRemoteEvents = remoteEvents.filter(event => event?.client_id !== status.deviceId)
        if (pending.length && foreignRemoteEvents.length) {
          const result = {
            kind: 'conflict',
            pushed: 0,
            remoteEvents,
            nextCursor: maxCursor(remoteEvents),
            pendingCount: pending.length,
          }
          pendingRemoteCursor = null
          emit(result)
          return result
        }
        const pushedIds = []
        for (const operation of pending) {
          checkActive()
          let acknowledgement
          try {
            acknowledgement = await repository.pushOperation({
              workspaceId: status.workspaceId,
              deviceId: status.deviceId,
              operation,
            })
            checkActive()
          } catch (error) {
            checkActive()
            if (String(error?.code || '') !== '40001') throw error
            // 另一台设备可能恰好在本轮 pull 之后写入。服务端 CAS 拒绝本机快照后
            // 重新拉取，让调用方走与普通冲突完全相同的数据取舍流程。
            const racingEvents = await repository.pullChanges({
              workspaceId: status.workspaceId,
              cursor: status.cursor,
            })
            checkActive()
            const result = {
              kind: 'conflict',
              pushed: 0,
              remoteEvents: racingEvents,
              nextCursor: maxCursor(racingEvents),
              pendingCount: pending.length,
            }
            pendingRemoteCursor = null
            emit(result)
            return result
          }
          if (!acknowledgement || acknowledgement.operation_id !== operation.operationId) {
            throw new Error(`云端未确认操作：${operation.operationId}`)
          }
          pushedIds.push(operation.operationId)
        }
        if (pushedIds.length) await localApi.acknowledgeSync(pushedIds)
        checkActive()

        const result = {
          kind: 'ready',
          pushed: pushedIds.length,
          remoteEvents,
          nextCursor: maxCursor(remoteEvents),
        }
        pendingRemoteCursor = result.nextCursor
        emit(result)
        return result
      } catch (error) {
        if (stopped) return { kind: 'stopped', pushed: 0, remoteEvents: [] }
        const result = { kind: 'error', pushed: 0, remoteEvents: [], error }
        emit(result)
        return result
      } finally {
        running = false
      }
    },

    async commitRemoteCursor(cursor) {
      checkActive()
      if (!localApi || cursor === null || cursor === undefined) return null
      if (pendingRemoteCursor === null || String(cursor) !== String(pendingRemoteCursor)) {
        throw new Error('只能确认最近一次同步返回的远端游标')
      }
      const status = await localApi.getSyncStatus()
      checkActive()
      const current = status?.cursor === null || status?.cursor === undefined ? null : Number(status.cursor)
      const next = Number(cursor)
      if (!Number.isSafeInteger(next) || next < 0 || (current !== null && next < current)) {
        throw new Error(`同步游标不可回退：${cursor}`)
      }
      const result = await localApi.acknowledgeSync([], String(cursor))
      pendingRemoteCursor = null
      return result
    },
  }
}
