//#region ts/src/base.d.ts
type RouteMap = Record<string, {
  request: unknown;
  response: unknown;
}>;
interface RouteResponse {
  status: number;
  contentType: string | null;
  body?: unknown;
}
/** `[init?]` when every field of the request is optional, `[init]` otherwise. */
type Args<R> = {} extends R ? [init?: R] : [init: R];
/** 2xx -> true, everything else -> false. Gives `if (res.ok)` real narrowing. */
type Ok<S extends number> = `${S}` extends `2${string}` ? true : false;
declare namespace fetch_d_exports {
  export { Client$1 as Client, JsonResponse, Options$1 as Options, ResponseFor, TextResponse, UnknownResponse, client$1 as client };
}
interface Options$1 {
  baseUrl?: string;
  headers?: HeadersInit;
  fetch?: typeof globalThis.fetch;
}
type Client$1<T extends RouteMap> = <K extends keyof T>(route: K, ...args: Args<T[K]['request']>) => Promise<ResponseFor<T[K]['response']>>;
declare function client$1<T extends RouteMap>(options?: Options$1): Client$1<T>;
/** A response with content type `application/json` header. */
interface JsonResponse<S extends number, B> extends Response {
  readonly status: S;
  readonly ok: Ok<S>;
  json(): Promise<B>;
}
/** A response with content type `text/plain` header. */
interface TextResponse<S extends number> extends Response {
  readonly status: S;
  readonly ok: Ok<S>;
}
/** A response with no content type header often 204 and friends where there is no body. */
interface UnknownResponse<S extends number> extends Response {
  readonly status: S;
  readonly ok: Ok<S>;
}
type ResponseFor<R> = R extends RouteResponse ? [R['contentType']] extends [null] ? UnknownResponse<R['status']> : R['contentType'] extends `${string}json` ? JsonResponse<R['status'], R['body']> : TextResponse<R['status']> : never;
declare namespace xhr_d_exports {
  export { BodyOfStatus, Client, Options, RequestOf, ResponseOf, TypedXMLHttpRequest, client };
}
interface Options {
  baseUrl?: string;
  headers?: Record<string, string>;
  /** Called after `open`, before any `send`. */
  configure?: (xhr: XMLHttpRequest) => void;
}
type Client<T extends RouteMap> = <K extends keyof T & string>(route: K, ...args: Args<Omit<T[K]['request'], 'body'>>) => TypedXMLHttpRequest<T[K]['request'], T[K]['response']>;
declare function client<T extends RouteMap>(options?: Options): Client<T>;
interface TypedXMLHttpRequest<Req, Res> extends Omit<XMLHttpRequest, 'send' | 'status'> {
  /** 0 while unsent, and on network failure. */
  readonly status: Extract<Res, RouteResponse>['status'] | 0;
  send(body?: BodyOf<Req>): void;
  /** The response union as data discriminated by `status`. */
  result(): Promise<Res>;
}
type RequestOf<T extends RouteMap, K extends keyof T> = T[K]['request'];
type ResponseOf<T extends RouteMap, K extends keyof T, S extends number = number> = Extract<T[K]['response'], {
  status: S;
}>;
type BodyOfStatus<T extends RouteMap, K extends keyof T, S extends number> = Extract<T[K]['response'], {
  status: S;
}> extends {
  body: infer B;
} ? B : never;
type BodyOf<Req> = Req extends {
  body?: unknown;
} ? Req['body'] : never;
//#endregion
export { fetch_d_exports as Fetch, xhr_d_exports as Xhr };
//# sourceMappingURL=index.d.mts.map