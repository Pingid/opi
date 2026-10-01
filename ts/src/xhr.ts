import type { RouteMap, RouteInit, Args, RouteResponse } from './base.ts'
import { serializeQuery } from './base.ts'

// ------------------------------------------------------------------
// Typed XHR adapter
// ------------------------------------------------------------------

export interface Options {
  baseUrl?: string
  headers?: Record<string, string>
  /** Called after `open`, before any `send`. */
  configure?: (xhr: XMLHttpRequest) => void
}

export type Client<T extends RouteMap> = <K extends keyof T & string>(
  route: K,
  ...args: Args<Omit<T[K]['request'], 'body'>>
) => TypedXMLHttpRequest<T[K]['request'], T[K]['response']>

export function client<T extends RouteMap>(options: Options = {}): Client<T> {
  const { baseUrl = '', headers: baseHeaders, configure } = options

  const f = (route: string, init: RouteInit = {}) => {
    const sp = route.indexOf(' ')
    const method = route.slice(0, sp)
    let path = route.slice(sp + 1)

    if (init.params) {
      for (const k in init.params) {
        path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])))
      }
    }

    const qs = init.query ? serializeQuery(init.query) : ''
    const xhr = new XMLHttpRequest()
    xhr.open(method, `${baseUrl}${path}${qs && `?${qs}`}`)

    if (baseHeaders) for (const k in baseHeaders) xhr.setRequestHeader(k, baseHeaders[k]!)
    if (init.headers) {
      for (const k in init.headers) {
        if (init.headers[k] !== undefined) xhr.setRequestHeader(k, String(init.headers[k]))
      }
    }

    const ct: string | undefined = init.contentType
    if (ct && ct !== 'multipart/form-data') xhr.setRequestHeader('content-type', ct)

    const send = xhr.send.bind(xhr)
    ;(xhr as any).send = (body?: unknown) => {
      if (body === undefined) return send()
      if (!ct || ct === 'application/json') return send(JSON.stringify(body))
      if (ct === 'application/x-www-form-urlencoded') {
        return send(new URLSearchParams(body as Record<string, string>))
      }
      return send(body as XMLHttpRequestBodyInit)
    }

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
  result(): Promise<Res>
}

export type RequestOf<T extends RouteMap, K extends keyof T> = T[K]['request']

export type ResponseOf<T extends RouteMap, K extends keyof T, S extends number = number> = Extract<
  T[K]['response'],
  { status: S }
>

export type BodyOfStatus<T extends RouteMap, K extends keyof T, S extends number> =
  Extract<T[K]['response'], { status: S }> extends { body: infer B } ? B : never

type BodyOf<Req> = Req extends { body?: unknown } ? Req['body'] : never
