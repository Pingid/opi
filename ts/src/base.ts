export function serializeQuery(query: Record<string, unknown>): string {
  const sp = new URLSearchParams()
  for (const k in query) {
    const v = query[k]
    if (v === undefined) continue
    if (Array.isArray(v)) for (const item of v) sp.append(k, String(item))
    else if (typeof v === 'string') sp.set(k, v)
    else sp.set(k, JSON.stringify(v))
  }
  return sp.toString()
}

export type Route = { request: unknown; response: unknown }

export type RouteMap = Record<string, Route>

export interface RouteResponse {
  status: number
  contentType: string | null
  body?: unknown
}

export type RouteInit = {
  params?: Record<string, string>
  query?: Record<string, string>
  headers?: Record<string, string>
  body?: unknown
  contentType?: string
}

/** `[init?]` when every field of the request is optional, `[init]` otherwise. */
export type Args<R> = {} extends R ? [init?: R] : [init: R]

/** 2xx -> true, everything else -> false. Gives `if (res.ok)` real narrowing. */
export type Ok<S extends number> = `${S}` extends `2${string}` ? true : false
