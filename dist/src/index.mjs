import { t as __exportAll } from "../rolldown-runtime-D7D4PA-g.mjs";
//#region ts/src/base.ts
function serializeQuery(query) {
	const sp = new URLSearchParams();
	for (const k in query) {
		const v = query[k];
		if (v === void 0) continue;
		if (Array.isArray(v)) for (const item of v) sp.append(k, String(item));
		else if (typeof v === "string") sp.set(k, v);
		else sp.set(k, JSON.stringify(v));
	}
	return sp.toString();
}
//#endregion
//#region ts/src/req.ts
var req_exports = /* @__PURE__ */ __exportAll({ builder: () => builder });
const builder = (opts = {}) => {
	const { baseUrl = "", headers: baseheaders = {} } = opts;
	const url = (route, init = {}) => {
		const sp = route.indexOf(" ");
		let path = route.slice(sp + 1);
		if (init.params) for (const k in init.params) path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])));
		if (init.query) path += `?${serializeQuery(init.query)}`;
		return `${baseUrl}${path}`;
	};
	const headers = (init = {}) => {
		const hdrs = new Headers(baseheaders);
		if (init.headers) {
			for (const k in init.headers) if (init.headers[k] !== void 0) hdrs.set(k, String(init.headers[k]));
		}
		if (init.contentType) hdrs.set("content-type", init.contentType);
		return hdrs;
	};
	const body = (init = {}) => {
		let body;
		if (init.body !== void 0) {
			const ct = init.contentType;
			if (!ct || ct === "application/json") body = JSON.stringify(init.body);
			else if (ct === "application/x-www-form-urlencoded") body = new URLSearchParams(init.body);
			else if (ct === "multipart/form-data") body = init.body;
			else body = init.body;
		}
		return body;
	};
	const build = (route, init = {}) => {
		const method = route.slice(0, route.indexOf(" "));
		const hdrs = headers(init);
		const b = body(init);
		if (b && !init.contentType) hdrs.set("content-type", "application/json");
		return {
			url: url(route, init),
			method,
			body: b,
			headers: hdrs
		};
	};
	return {
		body,
		url,
		build
	};
};
//#endregion
//#region ts/src/fetch.ts
var fetch_exports = /* @__PURE__ */ __exportAll({
	bodyFetcher: () => bodyFetcher,
	fetcher: () => fetcher
});
const fetcher = (options = {}) => fetcherFor(options);
const bodyFetcher = (options = {}) => fetcherFor({
	...options,
	res: (r) => {
		if (!r.ok) return Promise.reject(/* @__PURE__ */ new Error(`Failed with status ${r.status}: ${r.statusText}`));
		if (r.headers.get("content-type")?.startsWith("application/json")) return r.json();
		if (r.headers.get("content-type")?.startsWith("text")) return r.text();
		return r.body;
	}
});
const fetcherFor = (options = {}) => {
	const { fetch: f = globalThis.fetch } = options;
	const res = options.res ?? ((r) => r);
	const build = builder(options);
	const fetchers = (route, init = {}) => f(build.url(route, init), build.build(route, init)).then((r) => res(r, route));
	return fetchers;
};
//#endregion
//#region ts/src/xhr.ts
var xhr_exports = /* @__PURE__ */ __exportAll({ client: () => client });
function client(options = {}) {
	const { configure } = options;
	const b = builder(options);
	const f = (route, init = {}) => {
		const rq = b.build(route, init);
		const xhr = new XMLHttpRequest();
		xhr.open(rq.method, rq.url);
		for (const [k, v] of rq.headers.entries()) xhr.setRequestHeader(k, v);
		const send = xhr.send.bind(xhr);
		xhr.send = (body) => send(b.body(body));
		xhr.result = () => new Promise((resolve, reject) => {
			xhr.addEventListener("error", () => reject(/* @__PURE__ */ new Error("Network error")));
			xhr.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")));
			xhr.addEventListener("load", () => {
				const contentType = xhr.getResponseHeader("content-type")?.split(";")[0] ?? null;
				const text = xhr.responseText;
				const isJson = contentType?.endsWith("json") ?? false;
				resolve({
					status: xhr.status,
					contentType,
					body: isJson && text ? JSON.parse(text) : text || void 0
				});
			});
		});
		configure?.(xhr);
		return xhr;
	};
	return f;
}
//#endregion
export { fetch_exports as Fetch, req_exports as Req, xhr_exports as Xhr };

//# sourceMappingURL=index.mjs.map