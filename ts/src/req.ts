/**
 *
 * @example
 *
 */

import type { RouteMap, RouteInit, Args } from './base.ts'
import { serializeQuery } from './base.ts'

export interface Builder<T extends RouteMap> {
  url: <K extends keyof T>(route: K, ...args: Args<Params<T[K]['request'], 'query' | 'params'>>) => string
  body: <K extends keyof T>(init: Params<T[K]['request'], 'body' | 'contentType'>) => BodyInit | undefined
  build: <K extends keyof T>(
    route: K,
    ...args: Args<Params<T[K]['request'], 'headers' | 'contentType'>>
  ) => { method: string; url: string; headers: Headers; body?: BodyInit }
}

export type RequestParts = {
  method: string
  url: string
  headers: Headers
  body?: BodyInit
}

type Params<T extends any, K extends keyof RouteInit> = K extends keyof T ? Pick<T, K> : T

export interface Options {
  baseUrl?: string
  headers?: HeadersInit
}

export const builder = <T extends RouteMap>(opts: Options = {}): Builder<T> => {
  const { baseUrl = '', headers: baseheaders = {} } = opts

  const url = (route: string, init: RouteInit) => {
    const sp = route.indexOf(' ')
    let path = route.slice(sp + 1)

    if (init.params)
      for (const k in init.params) path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])))

    return `${baseUrl}${path}${init.query && `?${serializeQuery(init.query)}`}`
  }

  const headers = (init: RouteInit) => {
    const hdrs = new Headers(baseheaders)
    if (init.headers)
      for (const k in init.headers) if (init.headers[k] !== undefined) hdrs.set(k, String(init.headers[k]))
    if (init.contentType) hdrs.set('content-type', init.contentType)
    return hdrs
  }

  const body = (init: RouteInit) => {
    let body: BodyInit | undefined
    if (init.body !== undefined) {
      const ct: string | undefined = init.contentType
      if (!ct || ct === 'application/json') {
        body = JSON.stringify(init.body)
      } else if (ct === 'application/x-www-form-urlencoded') {
        body = new URLSearchParams(init.body as Record<string, string>)
      } else if (ct === 'multipart/form-data') {
        // Let the runtime set the boundary.
        body = init.body as BodyInit
      } else {
        body = init.body as BodyInit
      }
    }

    return body
  }

  const build = (route: string, init: RouteInit) => {
    const method = route.slice(0, route.indexOf(' '))

    const hdrs = headers(init)
    const b = body(init)
    if (b && !init.contentType) hdrs.set('content-type', 'application/json')

    return { url: url(route, init), method, body: b, headers: hdrs }
  }

  const b: Builder<any> = { body, url: url as any, build: build as any }

  return b
}
