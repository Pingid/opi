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

import type { RouteMap, RouteInit, Args, Ok, RouteResponse, Route } from './base.ts'

import * as Req from './req.ts'

// ------------------------------------------------------------------
// Typed fetch adapter
// ------------------------------------------------------------------
export interface FetchOptions<R = Response> extends Req.Options {
  baseUrl?: string
  headers?: HeadersInit
  fetch?: typeof globalThis.fetch
  res?: (res: Response, route: string) => R | Promise<R>
}

export type Fetcher<T extends RouteMap> = <K extends keyof T>(
  route: K,
  ...args: Args<T[K]['request']>
) => Promise<Responds<T[K]>>

export const fetcher = <T extends RouteMap>(options: FetchOptions<Response> = {}): Fetcher<T> => fetcherFor(options)

// ------------------------------------------------------------------
// Typed fetch adapter
// ------------------------------------------------------------------
export interface FetchBodyOptions<T extends RouteMap = RouteMap> extends FetchOptions {
  res?: <K extends keyof T>(res: Response, route: K) => Promise<Body<T[K]['response']>>
}

export type FetchBody<T extends RouteMap> = <K extends keyof T>(
  route: K,
  ...args: Args<T[K]['request']>
) => Promise<Body<T[K]['response']>>

export type Body<R> = Extract<R, { status: 200 | 201 | 202 | 204; body?: any }>['body']

export const bodyFetcher = <T extends RouteMap>(options: FetchBodyOptions<T> = {}): FetchBody<T> =>
  fetcherFor({
    ...options,
    res: (r: Response) => {
      if (!r.ok) return Promise.reject(new Error(`Failed with status ${r.status}: ${r.statusText}`))
      if (r.headers.get('content-type')?.startsWith('application/json')) return r.json()
      if (r.headers.get('content-type')?.startsWith('text')) return r.text()
      return r.body
    },
  })

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

export type Responses<R> = R extends RouteResponse
  ? [R['contentType']] extends [null]
    ? UnknownResponse<R['status']>
    : R['contentType'] extends `${string}json`
      ? JsonResponse<R['status'], R['body']>
      : TextResponse<R['status']>
  : never

export type Responds<R extends Route> = Responses<R['response']>

// ------------------------------------------------------------------
// helper
// ------------------------------------------------------------------

const fetcherFor = (options: FetchOptions<any> = {}): Fetcher<any> => {
  const { fetch: f = globalThis.fetch } = options
  const res = options.res ?? ((r) => r)
  const build = Req.builder(options)
  const fetchers = (route: string, init: RouteInit = {}) =>
    f(build.url(route, init as any), build.build(route, init as any)).then((r) => res(r, route))
  return fetchers as unknown as Fetcher<any>
}
