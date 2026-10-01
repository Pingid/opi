/**
 *
 * @example
 * import { Routes } from './generated.opi.ts'
 *
 * const client = createClient<Routes>({
 *   baseUrl: 'https://api.example.com',
 *   headers: { 'Content-Type': 'application/json' },
 * })
 *
 * const response = await client('GET /users')
 */

import type { RouteMap, RouteInit, Args, Ok, RouteResponse } from './base.ts'
import { serializeQuery } from './base.ts'

// ------------------------------------------------------------------
// Typed fetch adapter
// ------------------------------------------------------------------

export interface Options {
  baseUrl?: string
  headers?: HeadersInit
  fetch?: typeof globalThis.fetch
}

export type Client<T extends RouteMap> = <K extends keyof T>(
  route: K,
  ...args: Args<T[K]['request']>
) => Promise<ResponseFor<T[K]['response']>>

export function client<T extends RouteMap>(options: Options = {}): Client<T> {
  const { baseUrl = '', fetch: f = globalThis.fetch, headers: baseHeaders } = options

  const fetchers = (route: string, init: RouteInit = {}) => {
    const sp = route.indexOf(' ')
    const method = route.slice(0, sp)
    let path = route.slice(sp + 1)

    if (init.params)
      for (const k in init.params) path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])))

    const qs = init.query ? serializeQuery(init.query) : ''
    const headers = new Headers(baseHeaders)
    if (init.headers)
      for (const k in init.headers) if (init.headers[k] !== undefined) headers.set(k, String(init.headers[k]))

    let body: BodyInit | undefined
    if (init.body !== undefined) {
      const ct: string | undefined = init.contentType
      if (!ct || ct === 'application/json') {
        headers.set('content-type', 'application/json')
        body = JSON.stringify(init.body)
      } else if (ct === 'application/x-www-form-urlencoded') {
        headers.set('content-type', ct)
        body = new URLSearchParams(init.body as Record<string, string>)
      } else if (ct === 'multipart/form-data') {
        // Let the runtime set the boundary.
        body = init.body as BodyInit
      } else {
        headers.set('content-type', ct)
        body = init.body as BodyInit
      }
    }

    return f(`${baseUrl}${path}${qs && `?${qs}`}`, { method, headers, body })
  }
  return fetchers as unknown as Client<T>
}

// ------------------------------------------------------------------
// Adapter Types
// ------------------------------------------------------------------

/** A response with content type `application/json` header. */
export interface JsonResponse<S extends number, B> extends Response {
  readonly status: S
  readonly ok: Ok<S>
  json(): Promise<B>
}

/** A response with content type `text/plain` header. */
export interface TextResponse<S extends number> extends Response {
  readonly status: S
  readonly ok: Ok<S>
}

/** A response with no content type header often 204 and friends where there is no body. */
export interface UnknownResponse<S extends number> extends Response {
  readonly status: S
  readonly ok: Ok<S>
}

export type ResponseFor<R> = R extends RouteResponse
  ? [R['contentType']] extends [null]
    ? UnknownResponse<R['status']>
    : R['contentType'] extends `${string}json`
      ? JsonResponse<R['status'], R['body']>
      : TextResponse<R['status']>
  : never
