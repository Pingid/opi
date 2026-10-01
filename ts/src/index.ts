export type Method = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'OPTIONS' | 'HEAD'

export interface OpenAPIOperation {
  method: Method
  path: string
  request: {
    body?: any
    contentType?: any
    params?: any
    query?: any
    headers?: any
    cookies?: any
  }
  response: Record<string, Record<number, any>>
}

// ------------------------------------------------------------------
// Fetch adapter
// ------------------------------------------------------------------
declare const OP: unique symbol

export interface FetchResponse<O> extends Response {
  [OP]: O
}

export interface FetchJsonResponse<O extends OpenAPIOperation, R, S extends number = number> extends FetchResponse<O> {
  readonly status: S
  json: () => Promise<R>
}

export type JsonFetchResponses<O extends OpenAPIOperation> = {
  [K in keyof O['response']['application/json']]: FetchJsonResponse<O, O['response']['application/json'][K], K & number>
}[keyof O['response']['application/json']]

type FetchResponses<O extends OpenAPIOperation> = [JsonFetchResponses<O>] extends [never]
  ? FetchResponse<O>
  : JsonFetchResponses<O>

export type FetchParams<T extends OpenAPIOperation['request']> = T['contentType'] extends 'application/json'
  ? Omit<T, 'contentType'> & { contentType?: T['contentType'] }
  : T

export interface FetchAdapterOptions<T extends Record<any, any>> {
  modify?: (url: string, req: FetchParams<T[keyof T]['request']>) => void
}

export const createFetch =
  <T extends Record<any, any>>(ftch: typeof fetch | FetchAdapterOptions<T> = globalThis.fetch) =>
  <K extends keyof T>(path: K, p: FetchParams<T[K]['request']>): Promise<FetchResponses<T[K]>> => {
    return {} as any
  }

// ------------------------------------------------------------------
// XHR adapter
// ------------------------------------------------------------------
export interface XhrRequest<O extends OpenAPIOperation> extends XMLHttpRequest {
  send: (p: O['request']['body']) => void
  json: () => Promise<XhrJson<O['response']['application/json']>>
}

export type XhrJson<O> = { [K in keyof O]: { status: K; data: O[K] } }[keyof O]

export interface XhrAdapterOptions<T extends Record<any, any>> {
  modify?: (xhr: XMLHttpRequest, req: XhrAdapterParams<T>) => void
  create?: (req: XhrAdapterParams<T>) => XMLHttpRequest
}
export type XhrAdapterParams<T extends Record<any, any>> = {
  [K in keyof T]: Omit<T[K]['request'], 'body'> & { url: string }
}[keyof T]

export const createXHR =
  <T extends Record<any, any>>(options?: XhrAdapterOptions<T>) =>
  <K extends keyof T>(
    path: K,
    p: Omit<T[K]['request'], 'contentType' | 'body'> & {
      contentType?: T[K]['request']['contentType']
    },
  ): XhrRequest<T[K]> => {
    return {} as any
  }

// ---------------- Util --------------------------
// type Intersect<U> = (U extends any ? (k: U) => void : never) extends (k: infer I) => void ? I : never
// type Compute<T> = { [K in keyof T]: T[K] } & {}
