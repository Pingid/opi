//#region ts/src/base.d.ts
type Route = {
  request: unknown;
  response: unknown;
};
type RouteMap = Record<string, Route>;
interface RouteResponse {
  status: number;
  contentType: string | null;
  body?: unknown;
}
type RouteInit = {
  params?: Record<string, string>;
  query?: Record<string, string>;
  headers?: Record<string, string>;
  body?: unknown;
  contentType?: string;
};
/** `[init?]` when every field of the request is optional, `[init]` otherwise. */
type Args<R> = {} extends R ? [init?: R] : [init: R];
/** 2xx -> true, everything else -> false. Gives `if (res.ok)` real narrowing. */
type Ok<S extends number> = `${S}` extends `2${string}` ? true : false;
declare namespace req_d_exports {
  export { Builder, Options$2 as Options, builder };
}
interface Builder<T extends RouteMap> {
  url: <K extends keyof T>(route: K, ...args: Args<Params<T[K]['request'], 'query' | 'params'>>) => string;
  body: <K extends keyof T>(init: Params<T[K]['request'], 'body' | 'contentType'>) => BodyInit | undefined;
  build: <K extends keyof T>(route: K, ...args: Args<Params<T[K]['request'], 'headers' | 'contentType'>>) => {
    method: string;
    url: string;
    headers: Headers;
    body?: BodyInit;
  };
}
type Params<T extends any, K extends keyof RouteInit> = K extends keyof T ? Pick<T, K> : T;
interface Options$2 {
  baseUrl?: string;
  headers?: HeadersInit;
}
declare const builder: <T extends RouteMap>(opts?: Options$2) => Builder<T>;
declare namespace fetch_d_exports {
  export { Fetcher, JsonResponse, Options$1 as Options, Responds, Responses, TextResponse, UnknownResponse, fetcher };
}
interface Options$1 extends Options$2 {
  baseUrl?: string;
  headers?: HeadersInit;
  fetch?: typeof globalThis.fetch;
}
type Fetcher<T extends RouteMap> = <K extends keyof T>(route: K, ...args: Args<T[K]['request']>) => Promise<Responds<T[K]>>;
declare const fetcher: <T extends RouteMap>(options?: Options$1) => Fetcher<T>;
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
type Responses<R> = R extends RouteResponse ? [R['contentType']] extends [null] ? UnknownResponse<R['status']> : R['contentType'] extends `${string}json` ? JsonResponse<R['status'], R['body']> : TextResponse<R['status']> : never;
type Responds<R extends Route> = Responses<R['response']>;
declare namespace xhr_d_exports {
  export { BodyOfStatus, Client, Options, RequestOf, ResponseOf, TypedXMLHttpRequest, client };
}
interface Options extends Options$2 {
  baseUrl?: string;
  headers?: Record<string, string>;
  /** Called after `open`, before any `send`. */
  configure?: (xhr: XMLHttpRequest) => void;
}
type Client<T extends RouteMap> = <K extends keyof T & string>(route: K, ...args: Args<OmitT<T[K]['request'], 'body'>>) => TypedXMLHttpRequest<T[K]['request'], T[K]['response']>;
type OmitT<T, K> = K extends keyof T ? Omit<T, K> : T;
declare function client<T extends RouteMap>(options?: Options): Client<T>;
interface TypedXMLHttpRequest<Req, Res> extends Omit<XMLHttpRequest, 'send' | 'status'> {
  /** 0 while unsent, and on network failure. */
  readonly status: Extract<Res, RouteResponse>['status'] | 0;
  send(body?: BodyOf<Req>): void;
  /** The response union as data discriminated by `status`. */
  await(): Promise<Res>;
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
export { fetch_d_exports as Fetch, req_d_exports as Req, xhr_d_exports as Xhr };
//# sourceMappingURL=index.d.mts.map