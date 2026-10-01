import { t as __exportAll } from "../rolldown-runtime-D7D4PA-g.mjs";
//#region ts/src/base.ts
function serializeQuery(query) {
	const sp = new URLSearchParams();
	for (const k in query) {
		const v = query[k];
		if (v === void 0 || v === null) continue;
		if (Array.isArray(v)) for (const item of v) sp.append(k, String(item));
		else sp.set(k, String(v));
	}
	return sp.toString();
}
//#endregion
//#region ts/src/fetch.ts
var fetch_exports = /* @__PURE__ */ __exportAll({ client: () => client$1 });
function client$1(options = {}) {
	const { baseUrl = "", fetch: f = globalThis.fetch, headers: baseHeaders } = options;
	const fetchers = (route, init = {}) => {
		const sp = route.indexOf(" ");
		const method = route.slice(0, sp);
		let path = route.slice(sp + 1);
		if (init.params) for (const k in init.params) path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])));
		const qs = init.query ? serializeQuery(init.query) : "";
		const headers = new Headers(baseHeaders);
		if (init.headers) {
			for (const k in init.headers) if (init.headers[k] !== void 0) headers.set(k, String(init.headers[k]));
		}
		let body;
		if (init.body !== void 0) {
			const ct = init.contentType;
			if (!ct || ct === "application/json") {
				headers.set("content-type", "application/json");
				body = JSON.stringify(init.body);
			} else if (ct === "application/x-www-form-urlencoded") {
				headers.set("content-type", ct);
				body = new URLSearchParams(init.body);
			} else if (ct === "multipart/form-data") body = init.body;
			else {
				headers.set("content-type", ct);
				body = init.body;
			}
		}
		return f(`${baseUrl}${path}${qs && `?${qs}`}`, {
			method,
			headers,
			body
		});
	};
	return fetchers;
}
//#endregion
//#region ts/src/xhr.ts
var xhr_exports = /* @__PURE__ */ __exportAll({ client: () => client });
function client(options = {}) {
	const { baseUrl = "", headers: baseHeaders, configure } = options;
	const f = (route, init = {}) => {
		const sp = route.indexOf(" ");
		const method = route.slice(0, sp);
		let path = route.slice(sp + 1);
		if (init.params) for (const k in init.params) path = path.replace(`{${k}}`, encodeURIComponent(String(init.params[k])));
		const qs = init.query ? serializeQuery(init.query) : "";
		const xhr = new XMLHttpRequest();
		xhr.open(method, `${baseUrl}${path}${qs && `?${qs}`}`);
		if (baseHeaders) for (const k in baseHeaders) xhr.setRequestHeader(k, baseHeaders[k]);
		if (init.headers) {
			for (const k in init.headers) if (init.headers[k] !== void 0) xhr.setRequestHeader(k, String(init.headers[k]));
		}
		const ct = init.contentType;
		if (ct && ct !== "multipart/form-data") xhr.setRequestHeader("content-type", ct);
		const send = xhr.send.bind(xhr);
		xhr.send = (body) => {
			if (body === void 0) return send();
			if (!ct || ct === "application/json") return send(JSON.stringify(body));
			if (ct === "application/x-www-form-urlencoded") return send(new URLSearchParams(body));
			return send(body);
		};
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
export { fetch_exports as Fetch, xhr_exports as Xhr };

//# sourceMappingURL=index.mjs.map