//#region ts/src/index.d.ts
export type Method = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'OPTIONS' | 'HEAD';
export interface OpenAPIOperation {
  method: Method;
  path: string;
  request: {
    body?: any;
    contentType?: any;
    params?: any;
    query?: any;
    headers?: any;
    cookies?: any;
  };
  response: Record<string, Record<number, any>>;
}
declare const OP: unique symbol;
export interface FetchResponse<O> extends Response {
  [OP]: O;
}
export interface FetchJsonResponse<O extends OpenAPIOperation, R, S extends number = number> extends FetchResponse<O> {
  readonly status: S;
  json: () => Promise<R>;
}
export type JsonFetchResponses<O extends OpenAPIOperation> = { [K in keyof O['response']['application/json']]: FetchJsonResponse<O, O['response']['application/json'][K], K & number>; }[keyof O['response']['application/json']];
type FetchResponses<O extends OpenAPIOperation> = [JsonFetchResponses<O>] extends [never] ? FetchResponse<O> : JsonFetchResponses<O>;
export type FetchParams<T extends OpenAPIOperation['request']> = T['contentType'] extends 'application/json' ? Omit<T, 'contentType'> & {
  contentType?: T['contentType'];
} : T;
export interface FetchAdapterOptions<T extends Record<any, any>> {
  modify?: (url: string, req: FetchParams<T[keyof T]['request']>) => void;
}
export declare const createFetch: <T extends Record<any, any>>(ftch?: typeof fetch | FetchAdapterOptions<T>) => <K extends keyof T>(path: K, p: FetchParams<T[K]['request']>) => Promise<FetchResponses<T[K]>>;
export interface XhrRequest<O extends OpenAPIOperation> extends XMLHttpRequest {
  send: (p: O['request']['body']) => void;
  json: () => Promise<XhrJson<O['response']['application/json']>>;
}
export type XhrJson<O> = { [K in keyof O]: {
  status: K;
  data: O[K];
}; }[keyof O];
export interface XhrAdapterOptions<T extends Record<any, any>> {
  modify?: (xhr: XMLHttpRequest, req: XhrAdapterParams<T>) => void;
  create?: (req: XhrAdapterParams<T>) => XMLHttpRequest;
}
export type XhrAdapterParams<T extends Record<any, any>> = { [K in keyof T]: Omit<T[K]['request'], 'body'> & {
  url: string;
}; }[keyof T];
export declare const createXHR: <T extends Record<any, any>>(options?: XhrAdapterOptions<T>) => <K extends keyof T>(path: K, p: Omit<T[K]['request'], 'contentType' | 'body'> & {
  contentType?: T[K]['request']['contentType'];
}) => XhrRequest<T[K]>;
//#endregion
//# sourceMappingURL=index.d.mts.map