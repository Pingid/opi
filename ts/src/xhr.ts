import type { RouteMap, RouteInit, Args, RouteResponse } from './base.ts'

import * as Req from './req.ts'

// ------------------------------------------------------------------
// Typed XHR adapter
// ------------------------------------------------------------------

export interface Options extends Req.Options {
  baseUrl?: string
  headers?: Record<string, string>
  /** Called after `open`, before any `send`. */
  configure?: (xhr: XMLHttpRequest) => void
}

export type Client<T extends RouteMap> = <K extends keyof T & string>(
  route: K,
  ...args: Args<OmitT<T[K]['request'], 'body'>>
) => TypedXMLHttpRequest<T[K]['request'], T[K]['response']>

type OmitT<T, K> = K extends keyof T ? Omit<T, K> : T

export function client<T extends RouteMap>(options: Options = {}): Client<T> {
  const { configure } = options
  const b = Req.builder(options)

  const f = (route: string, init: RouteInit = {}) => {
    const rq = b.build(route, init)
    const xhr = new XMLHttpRequest()
    xhr.open(rq.method, rq.url)

    for (const [k, v] of rq.headers.entries()) xhr.setRequestHeader(k, v)

    const send = xhr.send.bind(xhr)
    ;(xhr as any).send = (body?: unknown) => send(b.body(body) as XMLHttpRequestBodyInit)

    ;(xhr as any).result = () =>
      new Promise((resolve, reject) => {
        xhr.addEventListener('error', () => reject(new Error('Network error')))
        xhr.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')))
        xhr.addEventListener('load', () => {
          const contentType = xhr.getResponseHeader('content-type')?.split(';')[0] ?? null
          const text = xhr.responseText
          const isJson = contentType?.endsWith('json') ?? false
          resolve({
            status: xhr.status,
            contentType,
            body: isJson && text ? JSON.parse(text) : text || undefined,
          })
        })
      })

    configure?.(xhr)
    return xhr
  }
  return f as unknown as Client<T>
}

// ------------------------------------------------------------------
// Adapter Types
// ------------------------------------------------------------------

export interface TypedXMLHttpRequest<Req, Res> extends Omit<XMLHttpRequest, 'send' | 'status'> {
  /** 0 while unsent, and on network failure. */
  readonly status: Extract<Res, RouteResponse>['status'] | 0
  send(body?: BodyOf<Req>): void
  /** The response union as data discriminated by `status`. */
  await(): Promise<Res>
}

export type RequestOf<T extends RouteMap, K extends keyof T> = T[K]['request']

export type ResponseOf<T extends RouteMap, K extends keyof T, S extends number = number> = Extract<
  T[K]['response'],
  { status: S }
>

export type BodyOfStatus<T extends RouteMap, K extends keyof T, S extends number> =
  Extract<T[K]['response'], { status: S }> extends { body: infer B } ? B : never

type BodyOf<Req> = Req extends { body?: unknown } ? Req['body'] : never
